use async_trait::async_trait;
use dialoguer::{Input, Password};
use ethers::types::{Address, Chain, H160, U256};
use ivynet_macros::h160;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{copy, BufReader},
    path::PathBuf,
};
use tracing::{debug, info};
use zip::ZipArchive;

use crate::{
    avs::AvsVariant,
    config::{self, IvyConfig},
    eigen::{
        node_classes::{self, NodeClass},
        quorum::QuorumType,
    },
    env::edit_env_vars,
    error::IvyError,
};

#[derive(Default)]
pub struct AltLayer {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for AltLayer {
    async fn setup(&self, env_path: std::path::PathBuf) -> Result<(), IvyError> {
        download_operator_setup(env_path.clone()).await?;
        Ok(())
    }

    async fn build_env(&self, env_path: PathBuf, chain: Chain, config: &IvyConfig) -> Result<(), IvyError> {
        let run_script_path = env_path.join("operator_setup");
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
            let mut env_values: HashMap<&str, &str> = HashMap::new();
            let node_hostname = reqwest::get("https://api.ipify.org").await?.text().await?;
            env_values.insert("NODE_HOSTNAME", &node_hostname);

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

    fn validate_node_size(&self, _: U256, _: u32) -> Result<bool, IvyError> {
        let (_, _, disk_info) = config::get_system_information()?;
        let class = node_classes::get_node_class()?;
        // XL node + 50gb disk space
        Ok(class >= NodeClass::XL && disk_info >= 50000000000)
    }

    /// Currently, AltLayer Mach AVS is operating in allowlist mode only: https://docs.altlayer.io/altlayer-documentation/altlayer-facilitated-actively-validated-services/xterio-mach-avs-for-xterio-chain/operator-guide
    async fn optin(
        &self,
        _quorums: Vec<crate::eigen::quorum::QuorumType>,
        _eigen_path: std::path::PathBuf,
        _private_keyfile: PathBuf,
        _chain: Chain,
    ) -> Result<(), IvyError> {
        todo!()
    }

    /// Quorum stake requirements can be found in the AltLayer docs: https://docs.altlayer.io/altlayer-documentation/altlayer-facilitated-actively-validated-services/xterio-mach-avs-for-xterio-chain/operator-guide
    fn quorum_min(&self, chain: Chain, quorum_type: crate::eigen::quorum::QuorumType) -> U256 {
        match chain {
            Chain::Mainnet => match quorum_type {
                QuorumType::LST => U256::zero(),
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            Chain::Holesky => match quorum_type {
                QuorumType::LST => U256::from(10 ^ 18), // one ETH
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            _ => todo!("Unimplemented"),
        }
    }

    // TODO: Add Eigen quorum.
    fn quorum_candidates(&self, chain: Chain) -> Vec<crate::eigen::quorum::QuorumType> {
        match chain {
            Chain::Mainnet => vec![QuorumType::LST],
            Chain::Holesky => vec![QuorumType::LST],
            _ => todo!("Unimplemented"),
        }
    }

    /// AltLayer stake registry contracts: https://github.com/alt-research/mach-avs
    fn stake_registry(&self, chain: Chain) -> Address {
        match chain {
            Chain::Mainnet => h160!(0x49296A7D4a76888370CB377CD909Cc73a2f71289),
            Chain::Holesky => h160!(0x0b3eE1aDc2944DCcBb817f7d77915C7d38F7B858),
            _ => todo!("Unimplemented"),
        }
    }

    /// AltLayer registry coordinator contracts: https://github.com/alt-research/mach-avs
    fn registry_coordinator(&self, chain: Chain) -> Address {
        match chain {
            Chain::Mainnet => h160!(0x561be1AB42170a19f31645F774e6e3862B2139AA),
            Chain::Holesky => h160!(0x1eA7D160d325B289bF981e0D7aB6Bf3261a0FFf2),
            _ => todo!("Unimplemented"),
        }
    }
}

pub async fn download_operator_setup(eigen_path: PathBuf) -> Result<(), IvyError> {
    let mut setup = false;
    let temp_path = eigen_path.join("temp");
    let destination_path = eigen_path.join("operator_setup");
    if destination_path.exists() {
        //TODO: Doh! Prompting inside the library?
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
