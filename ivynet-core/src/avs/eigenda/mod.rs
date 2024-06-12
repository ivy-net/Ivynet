use dialoguer::{Input, Password};
use ethers::{
    signers::Signer,
    types::{Address, Chain, H160, U256},
};
use ivynet_macros::h160;
use std::{
    fmt::Display,
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
    config::{self, IvyConfig},
    download::dl_progress_bar,
    eigen::{
        node_classes::{self, NodeClass},
        quorum::QuorumType,
    },
    env_parser::EnvLines,
    error::IvyError,
    rpc_management::IvyProvider,
};
use async_trait::async_trait;

#[derive(Debug)]
pub enum CoreError {
    DownloadFailed,
}

impl Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::DownloadFailed => write!(f, "Failed to download resource"),
        }
    }
}

impl std::error::Error for CoreError {}

#[derive(ThisError, Debug)]
pub enum EigenDAError {
    #[error("Boot script failed: {0}")]
    ScriptError(String),
    #[error("Not eligible for Quorum: {0}")]
    QuorumValidationError(QuorumType),
}

pub struct EigenDA {}

impl EigenDA {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for EigenDA {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for EigenDA {
    // TODO: the env_path should probably be a constant or another constant-like attribute implemented
    // on the singleton struct.
    async fn setup(&self, env_path: PathBuf) -> Result<(), IvyError> {
        download_operator_setup(env_path.clone()).await?;
        download_g1_g2(env_path).await?;
        Ok(())
    }

    // TODO: method is far too complex, this should be compartmentalized so that we can be sure
    // that general eigenlayer envs are sufficiently decoupled from specific AVS envs
    async fn build_env(
        &self,
        env_path: PathBuf,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
    ) -> Result<(), IvyError> {
        let chain = Chain::try_from(provider.signer().chain_id())?;
        let rpc_url = config.get_rpc_url(chain)?;

        let run_script_path = env_path.join("eigenda_operator_setup");
        let run_script_path = match chain {
            Chain::Mainnet => run_script_path.join("mainnet"),
            Chain::Holesky => run_script_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };

        let mut set_vars: bool = false;

        let env_example_path = run_script_path.join(".env.example");
        let env_path = run_script_path.join(".env");
        if env_example_path.exists() && !env_path.exists() {
            std::fs::copy(env_example_path, env_path.clone())?;
            info!("Copied '.env.example' to '.env'.");
            set_vars = true;
        } else if !env_example_path.exists() {
            info!("The '.env.example' file does not exist.");
        } else {
            info!("The '.env' file already exists.");
            let reset_string: String = Input::new().with_prompt("Reset env file? (y/n)").interact_text()?;
            if reset_string == "y" {
                std::fs::remove_file(env_path.clone())?;
                std::fs::copy(env_example_path, env_path.clone())?;
                info!("Copied '.env.example' to '.env'.");
                set_vars = true;
            }
        }

        if set_vars {
            debug!("Setting env vars");
            let mut env_lines = EnvLines::load(&env_path)?;
            let node_hostname = reqwest::get("https://api.ipify.org").await?.text().await?;

            let home_dir = dirs::home_dir().expect("Could not get home directory");
            let home_str = home_dir.to_str().expect("Could not get home directory");

            let bls_key_name: String = Input::new()
            .with_prompt(
                "Input the name of your BLS key file - looks in .eigenlayer folder (where eigen cli stores the key)",
            )
            .interact_text()?;

            let mut bls_json_file_location = dirs::home_dir().expect("Could not get home dir");
            bls_json_file_location.push(".eigenlayer/operator_keys");
            bls_json_file_location.push(bls_key_name);
            bls_json_file_location.set_extension("bls.key.json");
            info!("BLS key file location: {:?}", bls_json_file_location);

            let bls_password: String =
                Password::new().with_prompt("Input the password for your BLS key file").interact()?;

            env_lines.set("NODE_HOSTNAME", &node_hostname);
            env_lines.set("NODE_CHAIN_RPC", &rpc_url);
            env_lines.set("USER_HOME", home_str);
            env_lines.set(
                "NODE_BLS_KEY_FILE_HOST",
                bls_json_file_location.to_str().expect("Could not get BLS key file location"),
            );
            env_lines.set("NODE_BLS_KEY_PASSWORD", &bls_password);
            env_lines.save(&env_path)?;
        }

        Ok(())
    }

    // TODO: This method may need to be abstracted in some way, as not all AVS types encforce
    // quorum_pericentage.
    fn validate_node_size(&self, quorum_percentage: U256, bandwidth: u32) -> Result<bool, IvyError> {
        let (_, _, disk_info) = config::get_system_information()?;
        let class = node_classes::get_node_class()?;

        let mut acceptable: bool = false;
        match quorum_percentage {
            x if x < U256::from(3) => {
                // NOTE: Should these be || operators?
                if class >= NodeClass::LRG || bandwidth >= 1 || disk_info >= 20000000000 {
                    acceptable = true;
                }
            }
            x if x < U256::from(20) => {
                if class >= NodeClass::XL || bandwidth >= 1 || disk_info >= 150000000000 {
                    acceptable = true;
                }
            }
            x if x < U256::from(100) => {
                if class >= NodeClass::FOURXL || bandwidth >= 3 || disk_info >= 750000000000 {
                    acceptable = true;
                }
            }
            x if x < U256::from(1000) => {
                if class >= NodeClass::FOURXL || bandwidth >= 25 || disk_info >= 4000000000000 {
                    acceptable = true;
                }
            }
            x if x > U256::from(2000) => {
                if class >= NodeClass::FOURXL || bandwidth >= 50 || disk_info >= 8000000000000 {
                    acceptable = true;
                }
            }
            _ => {}
        }
        Ok(acceptable)
    }

    async fn optin(
        &self,
        quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        private_keyfile: PathBuf,
        chain: Chain,
    ) -> Result<(), IvyError> {
        // TODO: This is a very inefficient clone.
        let quorum_str: Vec<String> = quorums.iter().map(|quorum| (*quorum as u8).to_string()).collect();
        let quorum_str = quorum_str.join(",");

        let run_script_path = eigen_path.join("eigenda_operator_setup");
        let run_script_path = match chain {
            Chain::Mainnet => run_script_path.join("mainnet"),
            Chain::Holesky => run_script_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };

        let env_path = run_script_path.join(".env");
        let current_dir = std::env::current_dir()?;
        let current_env_path = current_dir.join(".env");

        info!("{} | {}", env_path.display(), current_env_path.display());

        // Copy .env file to current directory
        std::fs::copy(env_path, &current_env_path)?;

        // TODO: This shouldn't happen here! We should already have a wallet in signer
        let ecdsa_password: String =
            Password::new().with_prompt("Input the password for your ECDSA key file for quorum opt-in").interact()?;

        let run_script_path = run_script_path.join("run.sh");

        info!("Booting quorums: {:#?}", quorums);

        debug!("{} |  {}", run_script_path.display(), quorum_str);

        let optin = Command::new("sh")
            .arg(run_script_path)
            .arg("--operation-type")
            .arg("opt-in")
            .arg("--node-ecdsa-key-file-host")
            .arg(private_keyfile)
            .arg("--node-ecdsa-key-password")
            .arg(ecdsa_password)
            .arg("--quorums")
            .arg(quorum_str)
            .status()?;

        // Delete .env file from current directory
        std::fs::remove_file(current_env_path)?;

        if optin.success() {
            Ok(())
        } else {
            Err(EigenDAError::ScriptError(optin.to_string()).into())
        }
    }

    // TODO: Should probably be a hashmap
    fn quorum_min(&self, chain: Chain, quorum_type: QuorumType) -> U256 {
        match chain {
            Chain::Mainnet => match quorum_type {
                QuorumType::LST => U256::from(96 * (10 ^ 18)),
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            Chain::Holesky => match quorum_type {
                QuorumType::LST => U256::from(96 * (10 ^ 18)),
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            _ => todo!("Unimplemented"),
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
            Chain::Mainnet => h160!(0x006124ae7976137266feebfb3f4d2be4c073139d),
            Chain::Holesky => h160!(0xBDACD5998989Eec814ac7A0f0f6596088AA2a270),
            _ => todo!("Unimplemented"),
        }
    }

    fn registry_coordinator(&self, chain: Chain) -> Address {
        match chain {
            Chain::Mainnet => h160!(0x0baac79acd45a023e19345c352d8a7a83c4e5656),
            Chain::Holesky => h160!(0x53012C69A189cfA2D9d29eb6F19B32e0A2EA3490),
            _ => todo!("Unimplemented"),
        }
    }
}

/// Downloads eigenDA node resources
pub async fn download_g1_g2(eigen_path: PathBuf) -> Result<(), IvyError> {
    let resources_dir = eigen_path.join("eigenda_operator_setup/resources");
    let g1_file_path = resources_dir.join("g1.point");
    let g2_file_path = resources_dir.join("g2.point.PowerOf2");
    if g1_file_path.exists() {
        info!("The 'g1.point' file already exists.");
    } else {
        info!("Downloading 'g1.point'  to {}", g1_file_path.display());
        dl_progress_bar("https://srs-mainnet.s3.amazonaws.com/kzg/g1.point", g1_file_path).await?;
    }
    if g2_file_path.exists() {
        info!("The 'g2.point.PowerOf2' file already exists.");
    } else {
        info!("Downloading 'g2.point.PowerOf2' ...");
        dl_progress_bar("https://srs-mainnet.s3.amazonaws.com/kzg/g2.point.powerOf2", g2_file_path).await?
    }
    Ok(())
}

pub async fn download_operator_setup(eigen_path: PathBuf) -> Result<(), IvyError> {
    let mut setup = false;
    let repo_url = "https://github.com/ivy-net/eigenda-operator-setup/archive/refs/heads/master.zip";
    let temp_path = eigen_path.join("temp");
    let destination_path = eigen_path.join("eigenda_operator_setup");
    if destination_path.exists() {
        let reset_string: String = Input::new()
            .with_prompt("The operator setup directory already exists. Redownload? (y/n)")
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
        let first_dir =
            std::fs::read_dir(&temp_path)?.filter_map(Result::ok).find(|entry| entry.file_type().unwrap().is_dir());
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
