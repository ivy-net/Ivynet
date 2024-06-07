use async_trait::async_trait;
use dialoguer::{Input, Password};
use ethers::{
    signers::Signer,
    types::{Address, Chain, H160, U256},
};
use ivynet_macros::h160;
use once_cell::sync::Lazy;
use std::{
    env,
    fs::{self, File},
    io::{copy, BufReader},
    path::PathBuf,
    process::Command,
    sync::Arc,
};
use tracing::{debug, info};
use zip::ZipArchive;

use crate::{
    avs::AvsVariant,
    config::{self, IvyConfig},
    constants::IVY_METADATA,
    eigen::{
        node_classes::{self, NodeClass},
        quorum::QuorumType,
    },
    env_parser::EnvLines,
    error::IvyError,
    rpc_management::IvyProvider,
};

pub static ALTLAYER_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let dir = dirs::home_dir().expect("Could not get a home directory");
    dir.join(".eigenlayer/altlayer") // TODO: This may not be correct as mach-avs uses a nested
                                     // file structure
});

pub struct AltLayer {
    path: PathBuf,
}

impl AltLayer {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Default for AltLayer {
    fn default() -> Self {
        Self::new(ALTLAYER_PATH.to_path_buf())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for AltLayer {
    async fn setup(&self) -> Result<(), IvyError> {
        download_operator_setup(self.path.clone()).await?;
        Ok(())
    }

    async fn build_env(&self, provider: Arc<IvyProvider>, config: &IvyConfig) -> Result<(), IvyError> {
        let chain = Chain::try_from(provider.signer().chain_id())?;
        let ecdsa_address = provider.address();

        let run_script_path = self.path.join("eigenda_operator_setup");
        let run_script_path = match chain {
            Chain::Mainnet => run_script_path.join("mainnet"),
            Chain::Holesky => run_script_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };

        let run_script_path = run_script_path.join("mach-avs/op-sepolia");

        let mut set_vars: bool = false;

        // env file
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

            let rpc_url = config.get_rpc_url(chain)?;

            let home_dir = dirs::home_dir().unwrap();
            let home_str = home_dir.to_str().expect("Could not get home directory");

            // TODO: Resolve
            debug!("ecdsa address: {:?}", ecdsa_address);

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

            let bls_password: String =
                Password::new().with_prompt("Input the password for your BLS key file").interact()?;

            env_lines.set("USER_HOME", home_str);
            env_lines.set("ETH_RPC_URL", &rpc_url);
            env_lines.set("OPERATOR_ECDSA_ADDRESS", &format!("{:?}", ecdsa_address));
            env_lines.set(
                "NODE_BLS_KEY_FILE_HOST",
                bls_json_file_location.to_str().expect("Could not get BLS key file location"),
            );
            env_lines.set("OPERATOR_BLS_KEY_PASSWORD", &bls_password);
            env_lines.save(&env_path)?; //TODO
        }

        // .env.opt
        set_vars = false;

        let example_env = ".env.opt-example";
        let env = ".env.opt";

        let env_example_path = run_script_path.join(example_env);
        let env_path = run_script_path.join(env);
        if env_example_path.exists() && !env_path.exists() {
            std::fs::copy(env_example_path, env_path.clone())?;
            info!("Copied {} to {}.", example_env, env);
            set_vars = true;
        } else if !env_example_path.exists() {
            info!("The {} file does not exist.", example_env);
        } else {
            info!("The {} file already exists.", env);
            let reset_string: String = Input::new().with_prompt("Reset env.opt file? (y/n)").interact_text()?;
            if reset_string == "y" {
                std::fs::remove_file(env_path.clone())?;
                std::fs::copy(env_example_path, env_path.clone())?;
                info!("Copied '.env.opt-example' to '.env'.");
                set_vars = true;
            }
        }

        if set_vars {
            debug!("Setting env vars");

            let mut env_lines = EnvLines::load(&env_path)?;

            let user_home = dirs::home_dir().unwrap();
            let user_home = user_home.to_str().expect("Could not get home directory");

            let bls_key_name: String = Input::new()
                .with_prompt(
                    "Input the name of your BLS key file - look in .eigenlayer folder (where eigen cli stores the key)",
                )
                .interact_text()?;

            let mut bls_json_file_location = dirs::home_dir().expect("Could not get home directory");
            bls_json_file_location.push(".eigenlayer/operator_keys");
            bls_json_file_location.push(bls_key_name);
            bls_json_file_location.set_extension("bls.key.json");
            info!("BLS key file location: {:?}", bls_json_file_location);

            let bls_password: String =
                Password::new().with_prompt("Input the password for your BLS key file").interact()?;
            let ecdsa_password: String =
                Password::new().with_prompt("Input the password for your ECDSA key file").interact()?;

            env_lines.set("METADATA_URI", IVY_METADATA);
            env_lines.set("USER_HOME", user_home);
            env_lines.set(
                "NODE_BLS_KEY_FILE_HOST",
                bls_json_file_location.to_str().expect("Could not get BLS key file location"),
            );
            env_lines.set(
                "NODE_ECDSA_KEY_FILE_HOST",
                config.default_private_keyfile.to_str().expect("Bad private key path"),
            );
            env_lines.set("OPERATOR_BLS_KEY_PASSWORD", &bls_password);
            env_lines.set("OPERATOR_ECDSA_KEY_PASSWORD", &ecdsa_password);
            env_lines.save(&env_path)?;
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
    async fn opt_in(
        &self,
        _quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        _private_keyfile: PathBuf,
        chain: Chain,
    ) -> Result<(), IvyError> {
        let run_path =
            eigen_path.join("operator_setup").join(chain.to_string().to_lowercase()).join("mach-avs/op-sepolia");
        info!("Opting in...");
        debug!("altlayer opt-in: {}", run_path.display());
        // WARN: Changing directory here may not be the best strategy.
        env::set_current_dir(&run_path)?;
        let run_path = run_path.join("run.sh");
        let optin = Command::new("sh").arg(run_path).arg("opt-in").status()?;
        if optin.success() {
            Ok(())
        } else {
            // TODO: Consider a more robust .into()
            Err(IvyError::CommandError(optin.to_string()))
        }
    }

    async fn opt_out(
        &self,
        _quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        _private_keyfile: PathBuf,
        chain: Chain,
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

    fn path(&self) -> PathBuf {
        self.path.clone()
    }

    async fn start(
        &self,
        quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        chain: Chain,
    ) -> Result<(), IvyError> {
        todo!()
    }
    async fn stop(
        &self,
        quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        chain: Chain,
    ) -> Result<(), IvyError> {
        todo!()
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
