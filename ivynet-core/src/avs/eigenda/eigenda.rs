use dialoguer::{Input, Password};
use ethers_core::types::{Address, U256};
use rpc_management::Network;
use std::error::Error;
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::{copy, BufReader},
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error as ThisError;
use tracing::{debug, error, info};
use zip::read::ZipArchive;

use crate::env::edit_env_vars;
use crate::{
    avs::AvsVariant,
    config::{self, CONFIG},
    download::dl_progress_bar,
    eigen::{
        node_classes::{self, NodeClass},
        quorum::QuorumType,
    },
    rpc_management,
};

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

impl AvsVariant for EigenDA {
    // TODO: the env_path should probably be a constant or another constant-like attribute implemented on the
    // singleton struct.
    async fn setup(&self, env_path: PathBuf) -> Result<(), Box<dyn Error>> {
        download_operator_setup_files(env_path.clone()).await?;
        download_g1_g2(env_path).await?;
        Ok(())
    }

    // TODO: method is far too complex, this should be compartmentalized so that we can be sure
    // that general eigenlayer envs are sufficiently decoupled from specific AVS envs
    async fn build_env(&self, env_path: PathBuf, network: Network) -> Result<(), Box<dyn Error>> {
        let run_script_path = env_path.join("eigenda_operator_setup");
        let run_script_path = match network {
            Network::Mainnet => run_script_path.join("mainnet"),
            Network::Holesky => run_script_path.join("holesky"),
            Network::Local => todo!("Unimplemented"),
        };

        let mut set_vars: bool = false;

        let env_example_path = run_script_path.join(".env.example");
        let env_path = run_script_path.join(".env");
        if env_example_path.exists() && !env_path.exists() {
            std::fs::copy(env_example_path, env_path.clone())?;
            println!("Copied '.env.example' to '.env'.");
            set_vars = true;
        } else if !env_example_path.exists() {
            println!("The '.env.example' file does not exist.");
        } else {
            println!("The '.env' file already exists.");
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
            let mut env_values: HashMap<&str, &str> = HashMap::new();
            let node_hostname = reqwest::get("https://api.ipify.org").await?.text().await?;
            env_values.insert("NODE_HOSTNAME", &node_hostname);

            let rpc_url = CONFIG.lock()?.get_rpc_url(network)?;
            env_values.insert("NODE_CHAIN_RPC", &rpc_url);

            let home_dir = dirs::home_dir().unwrap();
            let home_str = home_dir.to_str().expect("Could not get home directory");
            env_values.insert("USER_HOME", home_str);

            let bls_key_name: String = Input::new()
            .with_prompt(
                "Input the name of your BLS key file - looks in .eigenlayer folder (where eigen cli stores the key)",
            )
            .interact_text()?;

            let mut bls_json_file_location = dirs::home_dir().expect("Could not get home directory");
            bls_json_file_location.push(".eigenlayer/operator_keys");
            bls_json_file_location.push(bls_key_name);
            bls_json_file_location.set_extension("bls.key.json");
            info!("BLS key file location: {:?}", bls_json_file_location);
            env_values.insert(
                "NODE_BLS_KEY_FILE_HOST",
                bls_json_file_location.to_str().expect("Could not get BLS key file location"),
            );

            let bls_password: String =
                Password::new().with_prompt("Input the password for your BLS key file").interact()?;
            env_values.insert("NODE_BLS_KEY_PASSWORD", &bls_password);

            edit_env_vars(env_path.to_str().unwrap(), env_values)?;
        }

        Ok(())
    }

    // TODO: This method may need to be abstracted in some way, as not all AVS types encforce
    // quorum_pericentage.
    fn validate_node_size(&self, quorum_percentage: U256, bandwidth: u32) -> Result<bool, Box<dyn std::error::Error>> {
        let (_, _, disk_info) = config::get_system_information()?;
        let class = node_classes::get_node_class()?;

        let mut acceptable: bool = false;
        match quorum_percentage {
            x if x < U256::from(3) => {
                if class >= NodeClass::LRG || bandwidth >= 1 || disk_info >= 20000000000 {
                    acceptable = true
                }
            }
            x if x < U256::from(20) => {
                if class >= NodeClass::XL || bandwidth >= 1 || disk_info >= 150000000000 {
                    acceptable = true
                }
            }
            x if x < U256::from(100) => {
                if class >= NodeClass::FOURXL || bandwidth >= 3 || disk_info >= 750000000000 {
                    acceptable = true
                }
            }
            x if x < U256::from(1000) => {
                if class >= NodeClass::FOURXL || bandwidth >= 25 || disk_info >= 4000000000000 {
                    acceptable = true
                }
            }
            x if x > U256::from(2000) => {
                if class >= NodeClass::FOURXL || bandwidth >= 50 || disk_info >= 8000000000000 {
                    acceptable = true
                }
            }
            _ => {}
        }
        Ok(acceptable)
    }

    async fn optin(
        &self,
        quorums: Vec<QuorumType>,
        network: Network,
        eigen_path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: This is a very inefficient clone.
        let quorum_str: Vec<String> = quorums.iter().map(|quorum| (*quorum as u8).to_string()).collect();
        let quorum_str = quorum_str.join(",");

        let run_script_path = eigen_path.join("eigenda_operator_setup");
        let run_script_path = match network {
            Network::Mainnet => run_script_path.join("mainnet"),
            Network::Holesky => run_script_path.join("holesky"),
            Network::Local => todo!("Unimplemented"),
        };

        let env_path = run_script_path.join(".env");
        let current_dir = std::env::current_dir()?;
        let current_env_path = current_dir.join(".env");

        info!("{} | {}", env_path.display(), current_env_path.display());

        // Copy .env file to current directory
        std::fs::copy(env_path, &current_env_path)?;

        let ecdsa_password: String =
            Password::new().with_prompt("Input the password for your ECDSA key file for quorum opt-in").interact()?;

        let private_keyfile = &CONFIG.lock()?.default_private_keyfile;

        let run_script_path = run_script_path.join("run.sh");

        info!("Booting quorums: {:#?}", quorums);

        debug!("{} | {} | {}", run_script_path.display(), private_keyfile.display(), quorum_str);

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
            Err(Box::new(EigenDAError::ScriptError(optin.to_string())))
        }
    }

    // TODO: Should probably be a hashmap
    fn quorum_min(&self, network: Network, quorum_type: QuorumType) -> U256 {
        match network {
            Network::Mainnet => match quorum_type {
                QuorumType::LST => U256::from(96 * (10 ^ 18)),
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            Network::Holesky => match quorum_type {
                QuorumType::LST => U256::from(96 * (10 ^ 18)),
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            Network::Local => todo!("Unimplemented"),
        }
    }

    // TODO: Consider loading these from a TOML config file or somesuch
    fn quorum_candidates(&self, network: Network) -> Vec<QuorumType> {
        match network {
            Network::Mainnet => vec![QuorumType::LST],
            Network::Holesky => vec![QuorumType::LST],
            Network::Local => todo!("Unimplemented"),
        }
    }

    fn stake_registry(&self, network: Network) -> Address {
        match network {
            Network::Mainnet => "0x006124ae7976137266feebfb3f4d2be4c073139d".parse().unwrap(),
            Network::Holesky => "0xBDACD5998989Eec814ac7A0f0f6596088AA2a270".parse().unwrap(),
            Network::Local => todo!("Unimplemented"),
        }
    }

    fn registry_coordinator(&self, network: Network) -> Address {
        match network {
            Network::Mainnet => "0x0baac79acd45a023e19345c352d8a7a83c4e5656".parse().unwrap(),
            Network::Holesky => "0x53012C69A189cfA2D9d29eb6F19B32e0A2EA3490".parse().unwrap(),
            Network::Local => todo!("Unimplemented"),
        }
    }
}

/// Downloads eigenDA node resources
pub async fn download_g1_g2(eigen_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let resources_dir = eigen_path.join("eigenda_operator_setup/resources");
    let g1_file_path = resources_dir.join("g1.point");
    let g2_file_path = resources_dir.join("g2.point.PowerOf2");
    if g1_file_path.exists() {
        println!("The 'g1.point' file already exists.");
    } else {
        dl_progress_bar("https://srs-mainnet.s3.amazonaws.com/kzg/g1.point", g1_file_path).await?;
    }
    if g2_file_path.exists() {
        println!("The 'g2.point.PowerOf2' file already exists.");
    } else {
        println!("The 'g2.point.PowerOf2' file does not exist, downloading appropriate file");
        dl_progress_bar("https://srs-mainnet.s3.amazonaws.com/kzg/g2.point.powerOf2", g2_file_path).await?
    }

    Ok(())
}

//Whole function needs to be cleaned up
pub async fn download_operator_setup_files(eigen_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut setup = false;
    let operator_setup_path = eigen_path.join("eigenda_operator_setup");
    if operator_setup_path.exists() {
        let reset_string: String = Input::new()
            .with_prompt("The 'extracted_files' directory already exists. Redownload? (y/n)")
            .interact_text()?;

        if reset_string == "y" {
            setup = true;
            fs::remove_dir_all(operator_setup_path.clone())?;
        }
    } else {
        info!("The 'extracted_files' directory does not exist, downloading to {}", operator_setup_path.display());
        setup = true;
    }

    if setup {
        info!("Downloading setup files to {}", operator_setup_path.display());
        let repo_url = "https://github.com/ivy-net/eigenda-operator-setup/archive/refs/heads/master.zip";
        let response = reqwest::get(repo_url).await?;

        let mut dest = {
            let fname = response
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .unwrap_or("eigenda_operator_setup.zip");

            File::create(fname)?
        };
        let bytes = response.bytes().await?;
        std::io::copy(&mut bytes.as_ref(), &mut dest)?;

        let reader = BufReader::new(File::open("eigenda_operator_setup.zip")?);
        let mut archive = ZipArchive::new(reader)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = eigen_path.join("setup_files").join(file.name());

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

        let extracted_files_dir = eigen_path.join("setup_files");
        let first_dir = std::fs::read_dir(&extracted_files_dir)?
            .filter_map(Result::ok)
            .find(|entry| entry.file_type().unwrap().is_dir());
        if let Some(first_dir) = first_dir {
            let old_folder_path = first_dir.path();
            let new_folder_path = eigen_path.join("eigenda_operator_setup");
            std::fs::rename(old_folder_path, new_folder_path)?;
        }

        // Delete the "extracted_files" directory
        if extracted_files_dir.exists() {
            std::fs::remove_dir_all(extracted_files_dir)?;
        }

        // Delete the "eigenda_operator_setup.zip" file
        let zip_file_path = Path::new("eigenda_operator_setup.zip");
        if zip_file_path.exists() {
            std::fs::remove_file(zip_file_path)?;
        }
    }

    Ok(())
}
