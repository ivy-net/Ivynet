use async_trait::async_trait;
use core::str;
use dialoguer::{Input, Password};
use dirs::home_dir;
use ethers::{
    signers::Signer,
    types::{Address, Chain, U256},
};
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
    config::{self, IvyConfig},
    dockercmd::{docker_cmd, docker_cmd_status},
    download::dl_progress_bar,
    eigen::{
        contracts::delegation_manager::DelegationManagerAbi,
        node_classes::{self, NodeClass},
        quorum::{Quorum, QuorumType},
    },
    env_parser::EnvLines,
    error::{IvyError, SetupError},
    rpc_management::IvyProvider,
    utils::gb_to_bytes,
};

use self::contracts::StakeRegistryAbi;

mod contracts;

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
    path: PathBuf,
    chain: Chain,
    running: bool,
}

impl EigenDA {
    pub fn new(path: PathBuf, chain: Chain) -> Self {
        Self { path, chain, running: false }
    }

    pub fn new_from_chain(chain: Chain) -> Self {
        let home_dir = dirs::home_dir().unwrap();
        Self::new(home_dir.join(EIGENDA_PATH), chain)
    }
}

impl Default for EigenDA {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap();
        Self::new(home_dir.join(EIGENDA_PATH), Chain::Holesky)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for EigenDA {
    async fn setup(
        &self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
        _pw: Option<String>,
    ) -> Result<(), IvyError> {
        download_operator_setup(self.path.clone()).await?;
        download_g1_g2(self.path.clone()).await?;
        self.build_env(provider, config).await?;
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

    async fn start(&mut self) -> Result<Child, IvyError> {
        let docker_path = self.path.join("eigenda-operator-setup");
        let docker_path = match self.chain {
            Chain::Mainnet => docker_path.join("mainnet"),
            Chain::Holesky => docker_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };
        std::env::set_current_dir(docker_path.clone())?;
        debug!("docker start: {} ", docker_path.display());
        let build = docker_cmd_status(["build", "--no-cache"])?;

        let _ = docker_cmd_status(["config"])?;

        if !build.success() {
            return Err(EigenDAError::ScriptError(build.to_string()).into());
        }

        // NOTE: See the limitations of the Stdio::piped() method if this experiences a deadlock
        let cmd = docker_cmd(["up", "--force-recreate"])?;
        debug!("cmd PID: {:?}", cmd.id());
        self.running = true;
        Ok(cmd)
    }

    async fn attach(&mut self) -> Result<Child, IvyError> {
        //TODO: Make more robust once path from avs config file is integrated
        let setup_path =
            home_dir().unwrap().join(EIGENDA_PATH).join("eigenda-operator-setup/holesky");
        info!("Path: {:?}", &setup_path);
        std::env::set_current_dir(&setup_path)?;

        let cmd = docker_cmd(["logs", "-f"])?;

        self.running = true;
        Ok(cmd)
    }

    async fn stop(&mut self) -> Result<(), IvyError> {
        let docker_path = self.path.join("eigenda-operator-setup");
        let docker_path = match self.chain {
            Chain::Mainnet => docker_path.join("mainnet"),
            Chain::Holesky => docker_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };
        std::env::set_current_dir(docker_path)?;
        let _ = docker_cmd_status(["stop"])?;
        self.running = false;
        Ok(())
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }

    fn running(&self) -> bool {
        self.running
    }

    fn name(&self) -> &'static str {
        "eigenda"
    }

    async fn register(
        &self,
        provider: Arc<IvyProvider>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        keyfile_password: &str,
    ) -> Result<(), IvyError> {
        let quorums = self.get_bootable_quorums(provider.clone()).await?;
        if quorums.is_empty() {
            return Err(EigenDAError::NoBootableQuorumsError.into());
        }
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
            .arg("opt-in")
            .arg("--node-ecdsa-key-file-host")
            .arg(private_keypath)
            .arg("--node-ecdsa-key-password")
            .arg(keyfile_password)
            .arg("--quorums")
            .arg(quorum_str)
            .status()?;

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
            .status()?;

        if optin.success() {
            Ok(())
        } else {
            Err(EigenDAError::ScriptError(optin.to_string()).into())
        }
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
        let delegation_manager = DelegationManagerAbi::new(
            contracts::registry_coordinator(self.chain),
            provider.clone(),
        );
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
        &self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
    ) -> Result<(), IvyError> {
        let chain = Chain::try_from(provider.signer().chain_id())?;
        let rpc_url = config.get_rpc_url(chain)?;

        let avs_run_path = self.path.join("eigenda-operator-setup");
        let avs_run_path = match chain {
            Chain::Mainnet => avs_run_path.join("mainnet"),
            Chain::Holesky => avs_run_path.join("holesky"),
            _ => todo!("Unimplemented"),
        };

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
        let bls_key_name: String = Input::new()
            .with_prompt(
                "Input the name of your BLS key file without file extensions - looks in .eigenlayer folder (where eigen cli stores the key)",
            )
            .interact_text()?;

        let mut bls_json_file_location = dirs::home_dir().expect("Could not get home dir");
        bls_json_file_location.push(".eigenlayer/operator_keys");
        bls_json_file_location.push(bls_key_name);
        bls_json_file_location.set_extension("bls.key.json");
        debug!("BLS key file location: {:?}", bls_json_file_location);

        // TODO: Remove prompting
        let bls_password: String =
            Password::new().with_prompt("Input the password for your BLS key file").interact()?;

        env_lines.set(
            "NODE_BLS_KEY_FILE_HOST",
            bls_json_file_location.to_str().expect("Could not get BLS key file location"),
        );
        env_lines.set("NODE_BLS_KEY_PASSWORD", &format!("'{}'", bls_password));
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
