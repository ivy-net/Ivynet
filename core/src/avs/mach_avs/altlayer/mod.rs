use async_trait::async_trait;
use dialoguer::{Input, Password};
use ethers::{
    signers::Signer,
    types::{Chain, U256},
};
use std::{
    env,
    fs::{self, File},
    io::{copy, BufReader},
    path::PathBuf,
    process::Command,
    sync::Arc,
};
use tracing::{debug, error, info};
use zip::ZipArchive;

use crate::{
    avs::{names::AvsName, AvsVariant},
    config::{self, IvyConfig},
    constants::IVY_METADATA,
    docker::log::CmdLogSource,
    eigen::{
        node_classes::{self, NodeClass},
        quorum::QuorumType,
    },
    env_parser::EnvLines,
    error::{IvyError, SetupError},
    keychain::{KeyType, Keychain},
    rpc_management::IvyProvider,
    utils::gb_to_bytes,
};

const ALTLAYER_PATH: &str = ".eigenlayer/altlayer";
const ALTLAYER_REPO_URL: &str =
    "https://github.com/alt-research/mach-avs-operator-setup/archive/refs/heads/master.zip";

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AltLayer {
    base_path: PathBuf,
    chain: Chain,
    running: bool,
}

impl AltLayer {
    pub fn new(base_path: PathBuf, chain: Chain) -> Self {
        Self { base_path, chain, running: false }
    }

    pub fn new_from_chain(chain: Chain) -> Self {
        let home_dir = dirs::home_dir().unwrap();
        Self::new(home_dir.join(ALTLAYER_PATH), chain)
    }
}

impl Default for AltLayer {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap();
        Self::new(home_dir.join(ALTLAYER_PATH), Chain::Holesky)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for AltLayer {
    async fn setup(
        &mut self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
        _pw: Option<String>,
        _is_custom: bool,
    ) -> Result<(), IvyError> {
        download_operator_setup(self.base_path.clone()).await?;
        self.build_env(provider, config).await?;
        Ok(())
    }

    fn validate_node_size(&self, _: U256) -> Result<bool, IvyError> {
        let (_, _, disk_info) = config::get_system_information()?;
        let class = node_classes::get_node_class()?;
        // XL node + 50gb disk space
        Ok(class >= NodeClass::XL && disk_info >= gb_to_bytes(50))
    }

    /// Currently, AltLayer Mach AVS is operating in allowlist mode only: https://docs.altlayer.io/altlayer-documentation/altlayer-facilitated-actively-validated-services/xterio-mach-avs-for-xterio-chain/operator-guide
    async fn register(
        &self,
        _provider: Arc<IvyProvider>,
        eigen_path: PathBuf,
        _private_keypath: PathBuf,
        _keyfile_password: &str,
    ) -> Result<(), IvyError> {
        let run_path = eigen_path
            .join("mach-avs-operator-setup")
            .join(self.chain.to_string().to_lowercase())
            .join("mach-avs/op-sepolia");
        info!("Opting in...");
        debug!("altlayer opt-in: {}", run_path.display());

        // WARN: Changing directory here may not be the best strategy.
        env::set_current_dir(&run_path)?;
        let run_path = run_path.join("run.sh");
        let optin = Command::new("sh").arg(run_path).arg("opt-in").status()?;
        if optin.success() {
            Ok(())
        } else {
            Err(IvyError::CommandError(optin.to_string()))
        }
    }

    async fn unregister(
        &self,
        _provider: Arc<IvyProvider>,
        _eigen_path: PathBuf,
        _private_keypath: PathBuf,
        _keyfile_password: &str,
    ) -> Result<(), IvyError> {
        todo!()
    }

    fn name(&self) -> &'static str {
        AvsName::AltLayer.as_str()
    }

    fn base_path(&self) -> PathBuf {
        self.base_path.clone()
    }

    async fn handle_log(&self, _line: &str, _src: CmdLogSource) -> Result<(), IvyError> {
        // TODO: Implement log handling
        Ok(())
    }

    fn run_path(&self) -> PathBuf {
        self.base_path
            .join("mach-avs-operator-setup")
            .join(self.chain.to_string().to_lowercase())
            .join("mach-avs/op-sepolia")
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }
}

impl AltLayer {
    /// Quorum stake requirements can be found in the AltLayer docs: https://docs.altlayer.io/altlayer-documentation/altlayer-facilitated-actively-validated-services/xterio-mach-avs-for-xterio-chain/operator-guide
    #[allow(dead_code)]
    fn quorum_min(&self, chain: Chain, quorum_type: crate::eigen::quorum::QuorumType) -> U256 {
        match chain {
            Chain::Mainnet => match quorum_type {
                QuorumType::LST => U256::zero(),
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            Chain::Holesky => match quorum_type {
                QuorumType::LST => U256::exp10(18), // one ETH
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            _ => todo!("Unimplemented"),
        }
    }

    // TODO: Add Eigen quorum.
    #[allow(dead_code)]
    fn quorum_candidates(&self, chain: Chain) -> Vec<crate::eigen::quorum::QuorumType> {
        match chain {
            Chain::Mainnet => vec![QuorumType::LST],
            Chain::Holesky => vec![QuorumType::LST],
            _ => todo!("Unimplemented"),
        }
    }

    async fn build_env(
        &self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
    ) -> Result<(), IvyError> {
        let chain = Chain::try_from(provider.signer().chain_id())?;
        let rpc_url = config.get_rpc_url(chain)?;

        let mach_avs_path = self.base_path.join("mach-avs-operator-setup");
        let avs_run_path = match chain {
            Chain::Mainnet => mach_avs_path.join("mainnet"),
            Chain::Holesky => mach_avs_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };

        let avs_run_path = avs_run_path.join("mach-avs/op-sepolia");
        //
        // .env
        //

        let env_example_path = avs_run_path.join(".env.example");
        let env_path = avs_run_path.join(".env");

        if !env_example_path.exists() {
            error!("The '.env.example' file does not exist at {}. '.env.example' is used for .env templating, please ensure the operator-setup was downloaded to the correct location.", env_example_path.display());
            return Err(SetupError::NoEnvExample.into());
        }

        std::fs::copy(env_example_path, env_path.clone())?;

        let ecdsa_address = provider.address();
        debug!("using provider address {ecdsa_address:?}");

        debug!("configuring env...");
        let mut env_lines = EnvLines::load(&env_path)?;

        let home_dir = dirs::home_dir().unwrap();
        let home_str = home_dir.to_str().expect("Could not get home directory");

        let keychain = Keychain::default();
        let keyname = keychain.select_key(KeyType::Bls)?;

        let mut bls_json_file_location = dirs::home_dir().expect("Could not get home directory");
        bls_json_file_location.push(".eigenlayer/operator_keys");
        bls_json_file_location.push(keyname.to_string());
        bls_json_file_location.set_extension("bls.key.json");
        info!("BLS key file location: {:?}", &bls_json_file_location);

        let bls_password: String =
            Password::new().with_prompt("Input the password for your BLS key file").interact()?;

        let p = keychain.get_path(keyname);
        let _ = fs::copy(p, &bls_json_file_location);
        let node_cache_path = mach_avs_path.join("resources/cache");

        env_lines.set("USER_HOME", home_str);
        env_lines.set("ETH_RPC_URL", &rpc_url);
        env_lines.set("OPERATOR_ECDSA_ADDRESS", &format!("{:?}", ecdsa_address));
        env_lines.set(
            "NODE_BLS_KEY_FILE_HOST",
            bls_json_file_location.to_str().expect("Could not get BLS key file location"),
        );
        env_lines.set("OPERATOR_BLS_KEY_PASSWORD", &bls_password);
        env_lines
            .set("NODE_CACHE_PATH_HOST", node_cache_path.to_str().expect("Could not parse string"));
        env_lines.save(&env_path)?;

        //
        // .env.opt
        //

        let env_example_path = avs_run_path.join(".env.opt-example");
        let env_path = avs_run_path.join(".env.opt");

        std::fs::copy(env_example_path, env_path.clone())?;

        debug!("Setting env.opt vars");
        let mut env_lines = EnvLines::load(&env_path)?;

        let ecdsa_password: String =
            Password::new().with_prompt("Input the password for your ECDSA key file").interact()?;

        env_lines.set("METADATA_URI", IVY_METADATA);
        env_lines.set("USER_HOME", home_str);
        env_lines.set(
            "NODE_BLS_KEY_FILE_HOST",
            bls_json_file_location.to_str().expect("Could not get BLS key file location"),
        );
        let keychain = Keychain::default();
        let keyname = keychain.select_key(KeyType::Ecdsa)?;
        let legacy_keyfile_path = keychain.get_path(keyname);
        env_lines.set(
            "NODE_ECDSA_KEY_FILE_HOST",
            legacy_keyfile_path.to_str().expect("Bad private key path"),
        );
        env_lines.set("OPERATOR_BLS_KEY_PASSWORD", &bls_password);
        env_lines.set("OPERATOR_ECDSA_KEY_PASSWORD", &ecdsa_password);
        env_lines.save(&env_path)?;
        Ok(())
    }
}

pub async fn download_operator_setup(eigen_path: PathBuf) -> Result<(), IvyError> {
    let mut setup = false;
    let temp_path = eigen_path.join("temp");
    let destination_path = eigen_path.join("mach-avs-operator-setup");
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
        let response = reqwest::get(ALTLAYER_REPO_URL).await?;

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
