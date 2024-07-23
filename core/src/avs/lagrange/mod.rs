/// ZK Coprocessor AVS handler.
/// Because the Lagrange ZK Coprocessor doesn't have a simple way to configure the network the AVS
/// is running on (Requires a combination of environment variables, and editing the docker-compose
/// file directly), this module handles the Lagrage directory somewhat differently, effectively
/// duplicating it per-network. E.G. `~/.eigenlayer/lagrange/holesky/lagrange-worker` and
/// `~/.eigenlayer/lagrange/mainnet/lagrange-worker`.
use async_trait::async_trait;
use dialoguer::Input;
use ethers::types::{Address, Chain, H160, U256};
use ivynet_macros::h160;
use std::{
    fs::{self, File},
    io::{copy, BufReader},
    path::PathBuf,
    process::{Child, Command},
    sync::Arc,
};
use thiserror::Error as ThisError;
use tracing::{debug, error, info};
use zip::read::ZipArchive;

use crate::{
    avs::AvsVariant,
    config::IvyConfig,
    dialog::get_confirm_password,
    eigen::quorum::QuorumType,
    env_parser::EnvLines,
    error::IvyError,
    io::{read_yaml, write_yaml},
    rpc_management::IvyProvider,
};

mod config;
mod docker_compose;

/**
*
*   General process for setting up the Lagrange AVS:
*   Create a lagrange key (No ecdsa dependencies)
*   Copy the ecdsa key to the lagrange-worker/config path (priv_key.json)
*   Register the lagrange key + priv_key
*   Remove priv_key
*   Start the docker container
*
*/

pub const LAGRANGE_PATH: &str = ".eigenlayer/lagrange";

#[derive(ThisError, Debug)]
pub enum LagrangeError {
    #[error("Boot script failed: {0}")]
    ScriptError(String),
    #[error("Failed to download resource: {0}")]
    DownloadFailedError(String),
    #[error("Keyfile password not found")]
    KeyfilePasswordNotFound,
}

#[derive(Debug, Clone)]
pub struct Lagrange {
    path: PathBuf,
    #[allow(dead_code)]
    chain: Chain,
    running: bool,
}

impl Lagrange {
    pub fn new(path: PathBuf, chain: Chain) -> Self {
        Self { path, chain, running: false }
    }

    pub fn new_from_chain(chain: Chain) -> Self {
        let home_dir = dirs::home_dir().unwrap();
        Self::new(home_dir.join(LAGRANGE_PATH), chain)
    }
}

impl Default for Lagrange {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap();
        let chain_dir = home_dir.join(LAGRANGE_PATH).join("holesky");
        Self::new(chain_dir, Chain::Holesky)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for Lagrange {
    // TODO: This currently creates a new Lagrange key every time it is run; this may be undesirable.
    // Figure out if this behavior needs to be stabilized.
    async fn setup(
        &self,
        _provider: Arc<IvyProvider>,
        config: &IvyConfig,
        _keyfile_pw: Option<String>,
    ) -> Result<(), IvyError> {
        download_operator_setup(self.path.clone()).await?;

        println!("Entering Lagrange keyfile password setup...");
        let lagrange_keyfile_pw = get_confirm_password();
        self.build_env(lagrange_keyfile_pw).await?;
        self.config_docker_compose(config).await?;
        generate_lagrange_key(self.path.clone()).await?;

        // copy ecdsa keyfile to lagrange-worker path
        let keyfile = config.default_private_keyfile.clone();
        let dest_file = self.run_path().join("config/priv_key.json");
        fs::copy(keyfile, dest_file)?;

        Ok(())
    }

    fn validate_node_size(&self, _quorum_percentage: U256) -> Result<bool, IvyError> {
        todo!()
    }

    async fn start(
        &mut self,
        _quorums: Vec<QuorumType>,
        _chain: Chain,
        keyfile_pw: Option<String>,
    ) -> Result<Child, IvyError> {
        if let Some(keyfile_pw) = keyfile_pw {
            std::env::set_var("AVS__ETH_PWD", keyfile_pw);
        } else {
            return Err(LagrangeError::KeyfilePasswordNotFound.into());
        }
        std::env::set_current_dir(self.run_path())?;
        debug!("docker start: {}", self.run_path().display());
        // NOTE: See the limitations of the Stdio::piped() method if this experiences a deadlock
        let cmd =
            Command::new("docker").arg("compose").arg("up").arg("--force-recreate").spawn()?;
        debug!("cmd PID: {:?}", cmd.id());
        self.running = true;
        Ok(cmd)
    }

    async fn stop(&mut self, _chain: Chain) -> Result<(), IvyError> {
        std::env::set_current_dir(self.run_path())?;
        let _ = Command::new("docker").arg("compose").arg("stop").status()?;
        self.running = false;
        Ok(())
    }

    // TODO: Should probably be a hashmap
    fn quorum_min(&self, chain: Chain, quorum_type: QuorumType) -> U256 {
        match chain {
            _ => unimplemented!(),
        }
    }

    // TODO: Consider loading these from a TOML config file or somesuch
    // TODO: Add Eigen quorum
    fn quorum_candidates(&self, chain: Chain) -> Vec<QuorumType> {
        match chain {
            Chain::Mainnet => vec![QuorumType::LST],
            Chain::Holesky => vec![QuorumType::LST],
            _ => todo!("Unimplemented"),
        }
    }

    fn stake_registry(&self, chain: Chain) -> Address {
        match chain {
            Chain::Mainnet => h160!(0x8dcdCc50Cc00Fe898b037bF61cCf3bf9ba46f15C),
            Chain::Holesky => h160!(0xf724cDC7C40fd6B59590C624E8F0E5E3843b4BE4),
            _ => todo!("Unimplemented"),
        }
    }

    fn registry_coordinator(&self, chain: Chain) -> Address {
        match chain {
            // TODO: TEMP WHILE WE REWORK THIS STRUCT
            _ => h160!(0x00000000000000000000000000000000DeaDBeef),
        }
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }

    fn running(&self) -> bool {
        self.running
    }
}

impl Lagrange {
    /// Registers the lagrange private key with the lagrange network.
    pub fn register(&self, config: &IvyConfig, keyfile_pw: &str) -> Result<(), IvyError> {
        // Copy keyfile to current dir
        //let private_keyfile = config.default_private_keyfile.clone();
        //let dest_dir = self.run_path().join("config");
        //if !dest_dir.exists() {
        //    fs::create_dir_all(dest_dir.clone())?;
        //}
        //let dest_file = dest_dir.join("priv_key.json");

        //debug!("{}", dest_file.display());
        //fs::copy(private_keyfile, &dest_file)?;
        // Change dir to run docker file
        std::env::set_current_dir(self.run_path())?;
        // Set local env variable to pass password to docker
        std::env::set_var("AVS__ETH_PWD", keyfile_pw);
        let _ = Command::new("docker")
            .arg("compose")
            .arg("run")
            .args(["--rm", "worker", "avs", "register"])
            .status()?;
        //fs::remove_file(dest_file)?;
        Ok(())
    }

    /// Constructor function for Lagrange run dir path
    fn run_path(&self) -> PathBuf {
        self.path.join("lagrange-worker")
    }

    /// Builds the .env file for the Lagrange worker
    async fn build_env(&self, lagrange_keyfile_pw: String) -> Result<(), IvyError> {
        debug!("configuring env...");
        let env_path = self.run_path().join(".env");
        let mut env_lines = EnvLines::load(&env_path)?;
        env_lines.set("AVS__LAGR_PWD", &lagrange_keyfile_pw);
        env_lines.set("NETWORK", self.chain.as_ref());
        env_lines.save(&env_path)
    }

    /// Updates the docker-compose file with the correct RPC URL for the instance chain.
    async fn config_docker_compose(&self, config: &IvyConfig) -> Result<(), IvyError> {
        let docker_compose_path = self.run_path().join("docker-compose.yaml");
        let mut docker_compose: docker_compose::Services = read_yaml(&docker_compose_path)?;
        let rpc_url = config.get_rpc_url(self.chain)?;
        docker_compose.set_rpc_url(rpc_url);
        write_yaml(&docker_compose_path, &docker_compose)?;
        Ok(())
    }
}

pub async fn generate_lagrange_key(path: PathBuf) -> Result<(), IvyError> {
    let docker_path = path.join("lagrange-worker");
    std::env::set_current_dir(docker_path)?;
    let _ = Command::new("docker")
        .arg("compose")
        .arg("run")
        .args(["--rm", "worker", "avs", "new-key"])
        .status()?;
    Ok(())
}

pub async fn download_operator_setup(eigen_path: PathBuf) -> Result<(), IvyError> {
    let mut setup = false;
    let repo_url = "https://github.com/Lagrange-Labs/worker/archive/refs/heads/main.zip";
    let temp_path = eigen_path.join("temp");
    let destination_path = eigen_path.join("lagrange-worker");
    if destination_path.exists() {
        let reset_string: String = Input::new()
            .with_prompt("The setup directory already exists. Redownload? (y/n)")
            .interact_text()?;

        if reset_string == "y" {
            setup = true;
            fs::remove_dir_all(destination_path.clone())?;
            fs::create_dir_all(temp_path.clone())?;
        }
    } else {
        info!("The setup directory does not exist, downloading to {}", temp_path.display());
        fs::create_dir_all(temp_path.clone())?;
        setup = true;
    }

    if setup {
        info!("Downloading setup files to {}", temp_path.display());
        let response = reqwest::get(repo_url).await?;

        let fname = temp_path.join("source.zip");
        let mut dest = File::create(&fname)?;
        let bytes = response.bytes().await?;
        std::io::copy(&mut bytes.as_ref(), &mut dest)?;
        let reader = BufReader::new(File::open(fname)?);
        let mut archive = ZipArchive::new(reader)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = temp_path.join(file.name());
            debug!("Extracting to {}", outpath.display());

            if (file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = File::create(&outpath)?;
                copy(&mut file, &mut outfile)?;
            }
        }
        let first_dir = std::fs::read_dir(&temp_path)?
            .filter_map(Result::ok)
            .find(|entry| entry.file_type().unwrap().is_dir());
        if let Some(first_dir) = first_dir {
            let old_folder_path = first_dir.path();
            debug!("{}", old_folder_path.display());
            std::fs::rename(old_folder_path, destination_path)?;
        }
        // Delete the setup directory
        if temp_path.exists() {
            info!("Cleaning up setup directory...");
            std::fs::remove_dir_all(temp_path)?;
        }
    }

    Ok(())
}
