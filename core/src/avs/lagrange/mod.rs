/// ZK Coprocessor AVS handler.
/// Because the Lagrange ZK Coprocessor doesn't have a simple way to configure the network the
/// AVS is running on (Requires a combination of environment variables, and editing the
/// docker-compose file directly), this module handles the Lagrage directory somewhat
/// differently, effectively duplicating it per-network. E.G.
/// `~/.eigenlayer/lagrange/holesky/lagrange-worker` and `~/.eigenlayer/lagrange/mainnet/
/// lagrange-worker`.
use async_trait::async_trait;
use dialoguer::Input;
use ethers::types::{Chain, U256};
use std::{
    fs::{self, File},
    io::{copy, BufReader},
    path::PathBuf,
    process::Command,
    sync::Arc,
};
use thiserror::Error as ThisError;
use tracing::{debug, error, info};
use zip::read::ZipArchive;

use crate::{
    avs::AvsVariant,
    config::IvyConfig,
    dialog::get_confirm_password,
    docker::log::CmdLogSource,
    eigen::quorum::QuorumType,
    env_parser::EnvLines,
    error::{IvyError, SetupError},
    keychain::{KeyType, Keychain},
    rpc_management::IvyProvider,
};

use super::{config::AvsConfig, names::AvsName};

mod config;

/**
 *
 *   General process for setting up the Lagrange AVS:
 *   Create a lagrange key (No ecdsa dependencies)
 *   Copy the operator ecdsa key to the lagrange-worker/config path (priv_key.json)
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
    base_path: PathBuf,
    #[allow(dead_code)]
    chain: Chain,
    running: bool,
    avs_config: AvsConfig,
}

impl Lagrange {
    pub fn new(base_path: PathBuf, chain: Chain, avs_config: AvsConfig) -> Self {
        Self { base_path, chain, running: false, avs_config }
    }

    pub fn new_from_chain(chain: Chain) -> Self {
        let base_path = dirs::home_dir().expect("Could not get home directory").join(LAGRANGE_PATH);
        let avs_config = AvsConfig::load(AvsName::LagrangeZK.as_str())
            .expect("Could not load AVS config - go through setup");
        Self::new(base_path, chain, avs_config)
    }
}

impl Default for Lagrange {
    fn default() -> Self {
        let avs_config = AvsConfig::load(AvsName::LagrangeZK.as_str())
            .expect("Could not load AVS config - go through setup");
        Self::new(avs_config.get_path(Chain::Holesky), Chain::Holesky, avs_config)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for Lagrange {
    // TODO: This currently creates a new Lagrange key every time it is run; this may be
    // undesirable. Figure out if this behavior needs to be stabilized.
    async fn setup(
        &mut self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
        _keyfile_pw: Option<String>,
        is_custom: bool,
    ) -> Result<(), IvyError> {
        self.build_pathing(is_custom)?;
        download_operator_setup(self.base_path.clone()).await?;
        self.build_env(provider, config)?;
        generate_lagrange_key(self.run_path()).await?;

        // copy ecdsa keyfile to lagrange-worker path
        let keychain = Keychain::default();
        let keyname = keychain.select_key(KeyType::Ecdsa)?;
        let keyfile = keychain.get_path(keyname);
        let dest_file = self.run_path().join("config/priv_key.json");
        fs::copy(keyfile, dest_file)?;

        // Change worker ID
        let worker_id: String =
            Input::new().with_prompt("Please enter a worker ID").interact_text()?;

        change_worker_id(self.run_path(), worker_id)?;

        Ok(())
    }

    fn validate_node_size(&self, _quorum_percentage: U256) -> Result<bool, IvyError> {
        todo!()
    }

    /// Registers the lagrange private key with the lagrange network.
    async fn register(
        &self,
        _provider: Arc<IvyProvider>,
        _eigen_path: PathBuf,
        private_keypath: PathBuf,
        keyfile_password: &str,
    ) -> Result<(), IvyError> {
        // Copy keyfile to current dir
        let dest_dir = self.run_path().join("config");
        if !dest_dir.exists() {
            fs::create_dir_all(dest_dir.clone())?;
        }
        let dest_file = dest_dir.join("priv_key.json");

        debug!("{}", dest_file.display());
        fs::copy(private_keypath, &dest_file)?;
        // Change dir to run docker file
        std::env::set_current_dir(self.run_path())?;
        // Set local env variable to pass password to docker
        std::env::set_var("AVS__ETH_PWD", keyfile_password);
        let _ = Command::new("docker")
            .arg("compose")
            .arg("run")
            .args(["--rm", "worker", "avs", "register"])
            .status()?;
        fs::remove_file(dest_file)?;
        Ok(())
    }

    async fn unregister(
        &self,
        _provider: Arc<IvyProvider>,
        _eigen_path: PathBuf,
        _private_keypath: PathBuf,
        _keyfile_password: &str,
    ) -> Result<(), IvyError> {
        todo!("Lagrange hasn't implemented this yet")
    }

    async fn handle_log(&self, _line: &str, _src: CmdLogSource) -> Result<(), IvyError> {
        // TODO: Implement log handling
        Ok(())
    }

    fn name(&self) -> AvsName {
        AvsName::LagrangeZK
    }

    fn base_path(&self) -> PathBuf {
        self.base_path.clone()
    }

    fn run_path(&self) -> PathBuf {
        self.avs_config.get_path(self.chain)
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }
}

impl Lagrange {
    /// Builds the .env file for the Lagrange worker
    fn build_env(&self, _provider: Arc<IvyProvider>, config: &IvyConfig) -> Result<(), IvyError> {
        let env_example_path = self.run_path().join(".env.example");
        let env_path = self.run_path().join(".env");

        println!("Entering Lagrange keyfile password setup...");
        let lagrange_keyfile_pw = get_confirm_password();

        if !env_example_path.exists() {
            error!("The '.env.example' file does not exist at {}. '.env.example' is used for .env templating, please ensure the operator-setup was downloaded to the correct location.", env_example_path.display());
            return Err(SetupError::NoEnvExample.into());
        }
        std::fs::copy(env_example_path, env_path.clone())?;

        debug!("configuring env...");
        debug!("{}", env_path.display());
        let mut env_lines = EnvLines::load(&env_path)?;
        env_lines.set("AVS__LAGR_PWD", &lagrange_keyfile_pw);
        env_lines.set("LAGRANGE_RPC_URL", &config.get_rpc_url(self.chain)?);
        env_lines.set("NETWORK", self.chain.as_ref());
        env_lines.save(&env_path)
    }

    // TODO: Consider loading these from a TOML config file or somesuch
    // TODO: Add Eigen quorum
    #[allow(dead_code)]
    fn quorum_candidates(&self, chain: Chain) -> Vec<QuorumType> {
        match chain {
            Chain::Mainnet => vec![QuorumType::LST],
            Chain::Holesky => vec![QuorumType::LST],
            _ => todo!("Unimplemented"),
        }
    }

    fn build_pathing(&mut self, is_custom: bool) -> Result<(), IvyError> {
        let path = if !is_custom {
            self.base_path.join("lagrange-worker").join(self.chain.as_ref())
        } else {
            AvsConfig::ask_for_path()
        };

        self.avs_config.set_path(self.chain, path, is_custom);
        self.avs_config.store();

        Ok(())
    }
}

pub async fn generate_lagrange_key(path: PathBuf) -> Result<(), IvyError> {
    std::env::set_current_dir(path)?;
    let _ = Command::new("docker")
        .arg("compose")
        .arg("run")
        .args(["--rm", "worker", "avs", "new-key"])
        .status()?;
    Ok(())
}

// Change worker ID in worker-conf.toml file under /config
pub fn change_worker_id(path: PathBuf, worker_id: String) -> Result<(), IvyError> {
    let mut lag_config = config::LagrangeConfig::load(path.join("config/worker-conf.toml"))?;

    lag_config.avs.worker_id = worker_id;

    lag_config.store(path.join("config/worker-conf.toml"))?;
    Ok(())
}

pub async fn download_operator_setup(eigen_path: PathBuf) -> Result<(), IvyError> {
    let mut setup = false;
    let repo_url = "https://github.com/ivy-net/lagrange-worker/archive/refs/heads/main.zip";
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
