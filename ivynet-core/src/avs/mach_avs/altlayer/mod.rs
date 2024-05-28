use dialoguer::{Input, Password};
use ethers_core::types::U256;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{copy, BufReader};
use std::path::PathBuf;
use tracing::{debug, info};
use zip::ZipArchive;

use crate::avs::AvsVariant;
use crate::config::{self, CONFIG};
use crate::eigen::node_classes::{self, NodeClass};
use crate::eigen::quorum::QuorumType;
use crate::env::edit_env_vars;
use crate::rpc_management::Network;

#[derive(Default)]
pub struct AltLayer {}

impl AvsVariant for AltLayer {
    async fn setup(&self, env_path: std::path::PathBuf) -> Result<(), Box<dyn Error>> {
        download_operator_setup(env_path.clone()).await?;
        Ok(())
    }

    async fn build_env(
        &self,
        env_path: std::path::PathBuf,
        network: crate::rpc_management::Network,
    ) -> Result<(), Box<dyn Error>> {
        let run_script_path = env_path.join("operator_setup");
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

    fn validate_node_size(&self, _: U256, _: u32) -> Result<bool, Box<dyn std::error::Error>> {
        let (_, _, disk_info) = config::get_system_information()?;
        let class = node_classes::get_node_class()?;
        // XL node + 50gb disk space
        Ok(class >= NodeClass::XL && disk_info >= 50000000000)
    }

    /// Currently, AltLayer Mach AVS is operating in allowlist mode only: https://docs.altlayer.io/altlayer-documentation/altlayer-facilitated-actively-validated-services/xterio-mach-avs-for-xterio-chain/operator-guide
    async fn optin(
        &self,
        quorums: Vec<crate::eigen::quorum::QuorumType>,
        network: crate::rpc_management::Network,
        eigen_path: std::path::PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    /// Quorum stake requirements can be found in the AltLayer docs: https://docs.altlayer.io/altlayer-documentation/altlayer-facilitated-actively-validated-services/xterio-mach-avs-for-xterio-chain/operator-guide
    fn quorum_min(
        &self,
        network: crate::rpc_management::Network,
        quorum_type: crate::eigen::quorum::QuorumType,
    ) -> U256 {
        match network {
            Network::Mainnet => match quorum_type {
                QuorumType::LST => U256::zero(),
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            Network::Holesky => match quorum_type {
                QuorumType::LST => U256::from(10 ^ 18), // one ETH
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            Network::Local => todo!("Unimplemented"),
        }
    }

    // TODO: Add Eigen quorum.
    fn quorum_candidates(&self, network: crate::rpc_management::Network) -> Vec<crate::eigen::quorum::QuorumType> {
        match network {
            Network::Mainnet => vec![QuorumType::LST],
            Network::Holesky => vec![QuorumType::LST],
            Network::Local => todo!("Unimplemented"),
        }
    }

    /// AltLayer stake registry contracts: https://github.com/alt-research/mach-avs
    fn stake_registry(&self, network: crate::rpc_management::Network) -> ethers_core::types::Address {
        match network {
            Network::Mainnet => "0x49296A7D4a76888370CB377CD909Cc73a2f71289".parse().unwrap(),
            Network::Holesky => "0x0b3eE1aDc2944DCcBb817f7d77915C7d38F7B858".parse().unwrap(),
            Network::Local => todo!("Unimplemented"),
        }
    }

    /// AltLayer registry coordinator contracts: https://github.com/alt-research/mach-avs
    fn registry_coordinator(&self, network: crate::rpc_management::Network) -> ethers_core::types::Address {
        match network {
            Network::Mainnet => "0x561be1AB42170a19f31645F774e6e3862B2139AA".parse().unwrap(),
            Network::Holesky => "0x1eA7D160d325B289bF981e0D7aB6Bf3261a0FFf2".parse().unwrap(),
            Network::Local => todo!("Unimplemented"),
        }
    }
}

pub async fn download_operator_setup(eigen_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut setup = false;
    let temp_path = eigen_path.join("temp");
    let destination_path = eigen_path.join("operator_setup");
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
        let repo_url = "https://github.com/alt-research/mach-avs-operator-setup/archive/refs/heads/master.zip";
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
