use async_trait::async_trait;
use contracts::RegistryCoordinator;
use core::str;
use dialoguer::Input;
use dirs::home_dir;
use dotenvy::from_path;
use ethers::{
    signers::Signer,
    types::{Address, Chain, H160, U256},
};
use semver::Version;
use std::{
    env,
    fs::{self, File},
    io::{copy, BufReader, Write},
    path::PathBuf,
    sync::Arc,
};
use thiserror::Error as ThisError;
use tokio::process::{Child, Command};
use tracing::{debug, error, info, warn};
use zip::read::ZipArchive;

use crate::{
    avs::AvsVariant,
    config::{self, IvyConfig},
    docker::{
        dockercmd::DockerCmd,
        log::{open_logfile, CmdLogSource},
    },
    download::dl_progress_bar,
    eigen::{
        contracts::delegation_manager::DelegationManagerAbi,
        node_classes::{self, NodeClass},
        quorum::{Quorum, QuorumType},
    },
    env_parser::EnvLines,
    error::{IvyError, SetupError},
    keychain::Keychain,
    rpc_management::IvyProvider,
    utils::gb_to_bytes,
};

use self::{
    contracts::StakeRegistryAbi,
    log::{ansi_sanitization_regex, level_regex},
};

use super::{names::AvsName, AvsConfig};

mod contracts;
mod log;

pub const EIGENDA_PATH: &str = ".eigenlayer/eigenda";
pub const EIGENDA_SETUP_REPO: &str =
    "https://github.com/ivy-net/eigenda-operator-setup/archive/refs/heads/master.zip";

#[derive(ThisError, Debug)]
pub enum EigenDAError {
    #[error("Boot script failed: {0}")]
    ScriptError(String),
    #[error("Not eligible for Quorum: {0}")]
    QuorumValidationError(QuorumType),
    #[error("Failed to download resource: {0}")]
    DownloadFailedError(String),
    #[error("No bootable quorums found. Please check your operator shares.")]
    NoBootableQuorumsError,
}

#[derive(Debug, Clone)]
pub struct EigenDA {
    base_path: PathBuf,
    chain: Chain,
    running: bool,
    avs_config: AvsConfig,
}

impl EigenDA {
    pub fn new(base_path: PathBuf, chain: Chain, avs_config: AvsConfig) -> Self {
        Self { base_path, chain, running: false, avs_config }
    }

    pub fn new_from_chain(chain: Chain) -> Self {
        let base_path = dirs::home_dir().expect("Could not get home directory").join(EIGENDA_PATH);
        let avs_config = AvsConfig::load(AvsName::EigenDA.as_str())
            .expect("Could not load AVS config - go through setup");
        Self::new(base_path, chain, avs_config)
    }
}

impl Default for EigenDA {
    fn default() -> Self {
        let home_dir = dirs::home_dir().expect("Could not get home directory");
        let base_path = home_dir.join(EIGENDA_PATH);
        let avs_config = AvsConfig::load(AvsName::EigenDA.as_str())
            .expect("Could not load AVS config - go through setup");
        Self::new(base_path, Chain::Holesky, avs_config)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for EigenDA {
    async fn setup(
        &mut self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
        operator_address: H160,
        bls_key_name: &str,
        bls_key_password: &str,
        is_custom: bool,
    ) -> Result<(), IvyError> {
        self.build_pathing(operator_address, is_custom)?;
        if !is_custom {
            download_operator_setup(self.base_path.clone()).await?;
            download_g1_g2(self.base_path.clone()).await?;
            self.build_env(provider, config, operator_address, bls_key_name, bls_key_password)
                .await?
        }

        Ok(())
    }

    // TODO: This method may need to be abstracted in some way, as not all AVS types encforce
    // quorum_pericentage.
    fn validate_node_size(&self, quorum_percentage: U256) -> Result<bool, IvyError> {
        let (_, _, disk_info) = config::get_system_information()?;
        let class = node_classes::get_node_class()?;

        let mut acceptable: bool = false;
        match quorum_percentage {
            x if x < U256::from(3) => {
                // NOTE: Should these be || operators?
                if class >= NodeClass::LRG || disk_info >= gb_to_bytes(20) {
                    acceptable = true;
                }
            }
            x if x < U256::from(20) => {
                if class >= NodeClass::XL || disk_info >= gb_to_bytes(150) {
                    acceptable = true;
                }
            }
            x if x < U256::from(100) => {
                if class >= NodeClass::FOURXL || disk_info >= gb_to_bytes(750) {
                    acceptable = true;
                }
            }
            x if x < U256::from(1000) => {
                if class >= NodeClass::FOURXL || disk_info >= gb_to_bytes(4000) {
                    acceptable = true;
                }
            }
            x if x > U256::from(2000) => {
                if class >= NodeClass::FOURXL || disk_info >= gb_to_bytes(8000) {
                    acceptable = true;
                }
            }
            _ => {}
        }
        Ok(acceptable)
    }

    async fn attach(&mut self) -> Result<Child, IvyError> {
        //TODO: Make more robust once path from avs config file is integrated
        let setup_path =
            home_dir().unwrap().join(EIGENDA_PATH).join("eigenda-operator-setup/holesky");
        info!("Path: {:?}", &setup_path);
        std::env::set_current_dir(&setup_path)?;

        let cmd = DockerCmd::new().args(["logs", "-f"]).current_dir(&setup_path).spawn()?;

        self.running = true;
        Ok(cmd)
    }

    async fn register(
        &self,
        provider: Arc<IvyProvider>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        keyfile_password: &str,
    ) -> Result<(), IvyError> {
        println!("Resgistering the EigenDA operator");
        let quorums = self.get_bootable_quorums(provider.clone()).await?;
        if quorums.is_empty() {
            return Err(EigenDAError::NoBootableQuorumsError.into());
        }
        let quorum_str: Vec<String> =
            quorums.iter().map(|quorum| (*quorum as u8).to_string()).collect();
        let quorum_str = quorum_str.join(",");
        println!("Fetched quorums...");
        let run_script_dir = eigen_path.join("eigenda-operator-setup");
        let run_script_dir = match self.chain {
            Chain::Mainnet => run_script_dir.join("mainnet"),
            Chain::Holesky => run_script_dir.join("holesky"),
            _ => todo!("Unimplemented"),
        };

        // Child shell scripts may not run correctly if the current directory is not set to their
        // own path.
        std::env::set_current_dir(run_script_dir.clone())?;
        let run_script_path = run_script_dir.join("run.sh");

        info!("Booting quorums: {:#?}", quorums);
        debug!("{} |  {}", run_script_path.display(), quorum_str);

        let optin = Command::new("sh")
            .arg(run_script_path)
            .arg("--operation-type")
            .arg("opt-in")
            .arg("--node-ecdsa-key-file-host")
            .arg(private_keypath)
            .arg("--node-ecdsa-key-password")
            .arg(keyfile_password)
            .arg("--quorums")
            .arg(quorum_str)
            .status()
            .await?;

        if optin.success() {
            Ok(())
        } else {
            Err(EigenDAError::ScriptError(optin.to_string()).into())
        }
    }

    async fn unregister(
        &self,
        provider: Arc<IvyProvider>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        keyfile_password: &str,
    ) -> Result<(), IvyError> {
        let quorums = self.get_bootable_quorums(provider.clone()).await?;
        let quorum_str: Vec<String> =
            quorums.iter().map(|quorum| (*quorum as u8).to_string()).collect();
        let quorum_str = quorum_str.join(",");

        let run_script_dir = eigen_path.join("eigenda-operator-setup");
        let run_script_dir = match self.chain {
            Chain::Mainnet => run_script_dir.join("mainnet"),
            Chain::Holesky => run_script_dir.join("holesky"),
            _ => todo!("Unimplemented"),
        };

        // Child shell scripts may not run correctly if the current directory is not set to their
        // own path.
        std::env::set_current_dir(run_script_dir.clone())?;
        let run_script_path = run_script_dir.join("run.sh");

        info!("Booting quorums: {:#?}", quorums);
        debug!("{} |  {}", run_script_path.display(), quorum_str);

        let optin = Command::new("sh")
            .arg(run_script_path)
            .arg("--operation-type")
            .arg("opt-out")
            .arg("--node-ecdsa-key-file-host")
            .arg(private_keypath)
            .arg("--node-ecdsa-key-password")
            .arg(keyfile_password)
            .arg("--quorums")
            .arg(quorum_str)
            .status()
            .await?;

        if optin.success() {
            Ok(())
        } else {
            Err(EigenDAError::ScriptError(optin.to_string()).into())
        }
    }

    async fn handle_log(&self, log: &str, src: CmdLogSource) -> Result<(), IvyError> {
        println!("{}", log);
        let log = ansi_sanitization_regex().replace_all(log, "").to_string();
        let logfile_dir = AvsConfig::log_path(self.name().as_str(), self.chain.as_ref());
        match src {
            CmdLogSource::StdOut => {
                // write to logfile simply capturing all stdout output
                let all_logfile = logfile_dir.join("stdout.log");
                let mut file = open_logfile(&all_logfile)?;
                writeln!(file, "{}", log)?;
                let level = match level_regex().captures(&log) {
                    Some(caps) => caps.get(1).unwrap().as_str(),
                    None => "unknown-level",
                };
                let logfile_name = match level.to_lowercase().as_str() {
                    "err" => "error",
                    "wrn" => "warn",
                    "inf" => "info",
                    "dbg" => "debug",
                    _ => "unknown-level",
                };
                let logfile = logfile_dir.join(format!("{}.log", logfile_name));
                let mut file = open_logfile(&logfile)?;
                writeln!(file, "{}", log)?;
                Ok(())
            }
            CmdLogSource::StdErr => {
                // Write to logfile simply capturing all stderr output
                let all_logfile = logfile_dir.join("stderr.log");
                let mut file = open_logfile(&all_logfile)?;
                writeln!(file, "{}", log)?;
                Ok(())
            }
        }
    }

    fn name(&self) -> AvsName {
        AvsName::EigenDA
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

    fn version(&self) -> Result<semver::Version, IvyError> {
        let yaml_path = self.run_path().join("docker-compose.yml");
        let env_path = yaml_path.with_file_name(".env");

        from_path(env_path).ok();
        let yaml_str = std::fs::read_to_string(yaml_path)?;
        let yaml_str = env::vars()
            .fold(yaml_str, |acc, (key, val)| acc.replace(&format!("${{{}}}", key), &val));

        let data: serde_yaml::Value = serde_yaml::from_str(&yaml_str)?;

        let image_value = &data["services"]["da-node"]["image"];

        if let Some(image) = image_value.as_str() {
            let parts: Vec<&str> = image.split(':').collect();
            if parts.len() == 2 {
                let version = parts[1];
                let version = semver::Version::parse(version)?;
                return Ok(version);
            }
        } else {
            debug!("Error: Could not parse image version");
        }

        Ok(Version::new(0, 0, 0))
    }

    async fn active_set(&self, provider: Arc<IvyProvider>) -> bool {
        let address = self.avs_config.operator_address(self.chain);
        let registry_coordinator_contract =
            RegistryCoordinator::new(contracts::registry_coordinator(self.chain), provider);

        let status = registry_coordinator_contract.get_operator_status(address).await;
        if let Ok(stat) = status {
            match stat {
                0 => {
                    info!("Operator has never registered");
                    return false;
                }
                1 => {
                    info!("Operator is in the active set");
                    return true;
                }
                2 => {
                    warn!("Operator is not in the active set - deregistered");
                    return false;
                }
                _ => {
                    warn!("Operator status is unknown");
                    return false;
                }
            }
        }

        false
    }
}

impl EigenDA {
    pub async fn get_current_total_stake(
        &self,
        provider: Arc<IvyProvider>,
        quorum_type: u8,
    ) -> Result<u128, IvyError> {
        let stake_registry_contract =
            StakeRegistryAbi::new(contracts::stake_registry(self.chain), provider.clone());
        let total_stake = stake_registry_contract.get_current_total_stake(quorum_type).await?;
        Ok(total_stake)
    }

    // TODO: Check to see if the delegation manager is querying strategies or quorums, also see if
    // there's a more compact method for this (EG query all strategies at once, or a selected
    // quorum
    pub async fn get_operator_shares_for_strategies(
        &self,
        provider: Arc<IvyProvider>,
        strategies: Vec<Address>,
    ) -> Result<Vec<U256>, IvyError> {
        let delegation_manager =
            DelegationManagerAbi::new(contracts::delegation_manager(self.chain), provider.clone());
        let shares = delegation_manager.get_operator_shares(provider.address(), strategies).await?;
        Ok(shares)
    }

    async fn get_bootable_quorums(
        &self,
        provider: Arc<IvyProvider>,
    ) -> Result<Vec<QuorumType>, IvyError> {
        let mut quorums_to_boot: Vec<QuorumType> = Vec::new();
        let chain = Chain::try_from(provider.signer().chain_id()).unwrap_or_default();
        for quorum_type in self.quorum_candidates(chain).iter() {
            let quorum = Quorum::try_from_type_and_network(*quorum_type, chain)?;
            let strategies = quorum.to_addresses();
            let shares =
                self.get_operator_shares_for_strategies(provider.clone(), strategies).await?;
            let total_shares = shares.iter().fold(U256::from(0), |acc, x| acc + x); // This may be
            info!("Operator shares for quorum {}: {}", quorum_type, total_shares);
            // let quorum_total =
            //     self.get_current_total_stake(provider.clone(), *quorum_type as u8).await?;
            quorums_to_boot.push(*quorum_type);
            // TODO: Reintroduce this check somewhere
            // let quorum_percentage = total_shares * 10000 / (total_shares + quorum_total);
            // if self.avs()?.validate_node_size(quorum_percentage)? {
            //     quorums_to_boot.push(*quorum_type);
            // };
        }
        Ok(quorums_to_boot)
    }

    async fn build_env(
        &mut self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
        operator_address: H160,
        bls_key_name: &str,
        bls_key_password: &str,
    ) -> Result<(), IvyError> {
        let chain = Chain::try_from(provider.signer().chain_id())?;
        let rpc_url = config.get_rpc_url(chain)?;

        let avs_run_path = self.base_path.join("eigenda-operator-setup");
        let avs_run_path = match chain {
            Chain::Mainnet => avs_run_path.join("mainnet"),
            Chain::Holesky => avs_run_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };

        self.avs_config.set_path(chain, avs_run_path.clone(), operator_address, false);
        self.avs_config.store();

        let env_example_path = avs_run_path.join(".env.example");
        let env_path = avs_run_path.join(".env");

        if !env_example_path.exists() {
            error!("The '.env.example' file does not exist at {}. '.env.example' is used for .env templating, please ensure the operator-setup was downloaded to the correct location.", env_example_path.display());
            return Err(SetupError::NoEnvExample.into());
        }
        std::fs::copy(env_example_path, env_path.clone())?;

        debug!("configuring env...");
        let mut env_lines = EnvLines::load(&env_path)?;

        // Node hostname
        let node_hostname = reqwest::get("https://api.ipify.org").await?.text().await?;
        info!("Using node hostname: {node_hostname}");
        // env_lines.set("NODE_HOSTNAME", &node_hostname);

        // Node chain RPC
        env_lines.set("NODE_CHAIN_RPC", &rpc_url);

        // User home directory
        let home_dir = dirs::home_dir().expect("Could not get home directory");
        let home_str = home_dir.to_str().expect("Could not get home directory");
        env_lines.set("USER_HOME", home_str);
        // Node resource paths
        env_lines.set("NODE_G1_PATH_HOST", r#"${EIGENLAYER_HOME}/eigenda/resources/g1.point"#);
        env_lines
            .set("NODE_G2_PATH_HOST", r#"${EIGENLAYER_HOME}/eigenda/resources/g2.point.powerOf2"#);
        env_lines.set(
            "NODE_CACHE_PATH_HOST",
            r#"${EIGENLAYER_HOME}/eigenda/eigenda-operator-setup/resources/cache"#,
        );

        // BLS key
        let keychain = Keychain::default();
        let bls_json_file_location =
            keychain.get_path(crate::keychain::KeyName::Bls(bls_key_name.to_owned()));
        debug!("BLS key file location: {:?}", &bls_json_file_location);

        env_lines.set(
            "NODE_BLS_KEY_FILE_HOST",
            bls_json_file_location.to_str().expect("Could not get BLS key file location"),
        );
        env_lines.set("NODE_BLS_KEY_PASSWORD", &format!("'{}'", bls_key_password));
        env_lines.save(&env_path)?;
        info!(".env file saved to {}", env_path.display());

        Ok(())
    }

    // TODO: Should probably be a hashmap
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    fn quorum_candidates(&self, chain: Chain) -> Vec<QuorumType> {
        match chain {
            Chain::Mainnet => vec![QuorumType::LST],
            Chain::Holesky => vec![QuorumType::LST],
            _ => todo!("Unimplemented"),
        }
    }

    fn build_pathing(&mut self, operator_address: H160, is_custom: bool) -> Result<(), IvyError> {
        let path = if !is_custom {
            let setup = self.base_path.join("eigenda-operator-setup");
            match self.chain {
                Chain::Mainnet => setup.join("mainnet"),
                Chain::Holesky => setup.join("holesky"),
                _ => todo!("Unimplemented"),
            }
        } else {
            AvsConfig::ask_for_path()
        };

        self.avs_config.set_path(self.chain, path, operator_address, is_custom);
        self.avs_config.store();

        Ok(())
    }
}

/// Downloads eigenDA node resources
pub async fn download_g1_g2(eigen_path: PathBuf) -> Result<(), IvyError> {
    let resources_dir = eigen_path.join("resources");
    fs::create_dir_all(resources_dir.clone())?;
    let g1_file_path = resources_dir.join("g1.point");
    let g2_file_path = resources_dir.join("g2.point.powerOf2");
    if g1_file_path.exists() {
        info!("The 'g1.point' file already exists.");
    } else {
        info!("Downloading 'g1.point'  to {}", g1_file_path.display());
        dl_progress_bar("https://srs-mainnet.s3.amazonaws.com/kzg/g1.point", g1_file_path).await?;
    }
    if g2_file_path.exists() {
        info!("The 'g2.point.powerOf2' file already exists.");
    } else {
        info!("Downloading 'g2.point.powerOf2' ...");
        dl_progress_bar("https://srs-mainnet.s3.amazonaws.com/kzg/g2.point.powerOf2", g2_file_path)
            .await?
    }
    Ok(())
}

pub async fn download_operator_setup(eigen_path: PathBuf) -> Result<(), IvyError> {
    let mut setup = false;
    let repo_url =
        "https://github.com/ivy-net/eigenda-operator-setup/archive/refs/heads/master.zip";
    let temp_path = eigen_path.join("temp");
    let destination_path = eigen_path.join("eigenda-operator-setup");
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
