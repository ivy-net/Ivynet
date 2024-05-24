use dialoguer::{Input, Password};

use ethers::{
    signers::Signer,
    types::{Address, Chain, U256},
};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::{copy, BufReader},
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;
use tracing::{debug, error, info};
use zip::read::ZipArchive;

use super::{super::AvsConstants, eigenda_info::setup_registry_coordinator};
use crate::{
    avs::eigenda::eigenda_info::setup_stake_registry,
    config::{self, IvyConfig},
    download::dl_progress_bar,
    eigen::{
        delegation_manager::DelegationManager,
        node_classes::{self, NodeClass},
        quorum::{Quorum, QuorumType},
    },
    error::IvyError,
    rpc_management::{connect_provider, IvyProvider},
};

// TODO: Quorums are per GROUPING of strategies, not per-strategy. May differ between AVSs?
// https://docs.eigenlayer.xyz/eigenlayer/operator-guides/operator-introduction#quorums

#[derive(Error, Debug)]
pub enum EigenDAError {
    #[error("Boot script failed: {0}")]
    ScriptError(String),
    #[error("Not eligible for Quorum: {0}")]
    QuorumValidationError(QuorumType),
}

pub struct EigenDA {
    env_path: PathBuf,
}

impl EigenDA {
    pub fn new(env_path: PathBuf) -> Self {
        Self { env_path }
    }
}

impl EigenDA {
    // fn setup() -> {}
    // fn verify_install() -> {} // MD5 hashing stuff
    async fn validate_quorum() -> Result<(), EigenDAError> {
        Ok(())
    }

    pub async fn boot(&self, config: &IvyConfig, operator: Address, provider: &IvyProvider) -> Result<(), IvyError> {
        let chain = Chain::try_from(provider.signer().chain_id()).unwrap_or_default();
        let quorums = Self::get_bootable_quorums(config, chain, operator).await?;
        if quorums.is_empty() {
            error!("Could not launch EgenDA, no bootable quorums found. Exiting...");
            return Err(IvyError::NoQuorums);
        }

        fs::create_dir_all(&self.env_path)?;

        let status = get_operator_status(&provider, operator).await?;
        if status == 1 {
            //Check which quorums they're already in and register for the others they're eligible
            // for
        } else {
            //Register operator for all quorums they're eligible for
        }

        download_operator_setup_files(self.env_path.clone()).await?;
        download_g1_g2(self.env_path.clone()).await?;
        self.build_env_file(chain).await?;
        optin(quorums, chain, self.env_path.clone(), config.default_private_keyfile.clone()).await?;
        Ok(())
    }

    pub async fn get_bootable_quorums(
        config: &IvyConfig,
        chain: Chain,
        operator: Address,
    ) -> Result<Vec<QuorumType>, IvyError> {
        let mut quorums_to_boot: Vec<QuorumType> = Vec::new();
        let candidates = Self::QUORUM_CANDIDATES;
        for quorum_type in candidates.iter() {
            let quorum = Quorum::try_from_type_and_network(*quorum_type, chain)?;
            let provider = connect_provider(&config.get_rpc_url(chain)?, None)?;
            let delegation_manager = DelegationManager::new(&provider);
            let shares = delegation_manager.get_shares_for_quorum(operator, &quorum).await?;
            let total_shares = shares.iter().fold(U256::from(0), |acc, x| acc + x);
            info!("Operator shares for quorum {}: {}", quorum_type, total_shares);
            // TODO: This may be queriable as a one-off on the AVS stake registry.
            let stake_registry = setup_stake_registry(&provider);
            let quorum_total = stake_registry.get_current_total_stake(*quorum_type as u8).await?;
            let quorum_percentage = total_shares * 10000 / (total_shares + quorum_total);
            let bandwidth: u32 = Input::new().with_prompt("Input your bandwidth in mbps").interact_text()?;
            if validate_node_size(quorum_percentage, bandwidth)? {
                quorums_to_boot.push(*quorum_type);
            };
        }
        Ok(quorums_to_boot)
    }

    pub async fn build_env_file(&self, chain: Chain) -> Result<(), IvyError> {
        let run_script_path = self.env_path.join("eigenda_operator_setup");
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
                println!("Copied '.env.example' to '.env'.");
                set_vars = true;
            }
        }

        if set_vars {
            let mut env_values: HashMap<&str, &str> = HashMap::new();
            let node_hostname = reqwest::get("https://api.ipify.org").await?.text().await?;
            env_values.insert("NODE_HOSTNAME", &node_hostname);

            let config = IvyConfig::new();
            let rpc_url = config.get_rpc_url(chain)?;
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
            println!("BLS key file location: {:?}", bls_json_file_location);
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
}

fn validate_node_size(quorum_percentage: U256, bandwidth: u32) -> Result<bool, IvyError> {
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

pub async fn optin(
    quorums: Vec<QuorumType>,
    chain: Chain,
    eigen_path: PathBuf,
    private_keyfile: PathBuf,
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

    let ecdsa_password: String =
        Password::new().with_prompt("Input the password for your ECDSA key file for quorum opt-in").interact()?;

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
        Err(EigenDAError::ScriptError(optin.to_string()).into())
    }
}

fn edit_env_vars(filename: &str, env_values: HashMap<&str, &str>) -> Result<(), IvyError> {
    let contents = fs::read_to_string(filename)?;
    let new_contents = contents
        .lines()
        .map(|line| {
            let mut parts = line.splitn(2, '=');
            let key: &str = parts.next().unwrap();
            if let Some(value) = env_values.get(key) {
                format!("{}={}", key, value)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(filename, new_contents.as_bytes())?;
    Ok(())
}

/// Downloads eigenDA node resources
pub async fn download_g1_g2(eigen_path: PathBuf) -> Result<(), IvyError> {
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

//Whole function needs to be cleaned up
pub async fn download_operator_setup_files(eigen_path: PathBuf) -> Result<(), IvyError> {
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
                .and_then(|name| if name.is_empty() { None } else { None })
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

            if (&*file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(&p)?;
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
            std::fs::rename(&old_folder_path, &new_folder_path)?;
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

pub async fn get_operator_status(provider: &IvyProvider, addr: Address) -> Result<u8, IvyError> {
    let coordinator = setup_registry_coordinator(provider);
    let operator_details = coordinator.get_operator(addr).await?;
    Ok(operator_details.status)
}
