/// ZK Coprocessor AVS handler
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
    eigen::quorum::QuorumType,
    error::{IvyError, SetupError},
    rpc_management::IvyProvider,
};

mod config;

pub const LAGRANGE_PATH: &str = ".eigenlayer/lagrange";

#[derive(ThisError, Debug)]
pub enum LagrangeError {
    #[error("Boot script failed: {0}")]
    ScriptError(String),
    #[error("Failed to download resource: {0}")]
    DownloadFailedError(String),
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
        Self::new(home_dir.join(LAGRANGE_PATH), Chain::Holesky)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for Lagrange {
    // TODO: This currently creates a new Lagrange key every time it is run; this may be undesirable.
    // Figure out if this behavior needs to be stabilized.
    async fn setup(&self, provider: Arc<IvyProvider>, config: &IvyConfig) -> Result<(), IvyError> {
        download_operator_setup(self.path.clone()).await?;
        generate_lagrange_key(self.path.clone()).await?;
        self.build_env(provider, config).await?;
        Ok(())
    }

    async fn build_env(
        &self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
    ) -> Result<(), IvyError> {
        Ok(())
    }

    fn validate_node_size(&self, quorum_percentage: U256) -> Result<bool, IvyError> {
        todo!()
    }

    async fn start(&mut self, quorums: Vec<QuorumType>, chain: Chain) -> Result<Child, IvyError> {
        let quorum_str: Vec<String> =
            quorums.iter().map(|quorum| (*quorum as u8).to_string()).collect();
        let quorum_str = quorum_str.join(",");

        let docker_path = self.path.join("p");
        let docker_path = match chain {
            Chain::Mainnet => docker_path.join("mainnet"),
            Chain::Holesky => docker_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };
        std::env::set_current_dir(docker_path.clone())?;
        debug!("docker start: {} |  {}", docker_path.display(), quorum_str);
        let build =
            Command::new("docker").arg("compose").arg("build").arg("--no-cache").status()?;

        let _ = Command::new("docker").arg("compose").arg("config").status()?;

        if !build.success() {
            return Err(LagrangeError::ScriptError(build.to_string()).into());
        }

        // NOTE: See the limitations of the Stdio::piped() method if this experiences a deadlock
        let cmd =
            Command::new("docker").arg("compose").arg("up").arg("--force-recreate").spawn()?;
        debug!("cmd PID: {:?}", cmd.id());
        self.running = true;
        Ok(cmd)
    }

    async fn stop(&mut self, chain: Chain) -> Result<(), IvyError> {
        let docker_path = self.path.join("eigenda-operator-setup");
        let docker_path = match chain {
            Chain::Mainnet => docker_path.join("mainnet"),
            Chain::Holesky => docker_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };
        std::env::set_current_dir(docker_path)?;
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
    pub fn register(&self) -> Result<(), IvyError> {
        let _ = Command::new("docker")
            .arg("compose")
            .arg("run")
            .args(["--rm", "worker", "avs", "register"])
            .status()?;
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
