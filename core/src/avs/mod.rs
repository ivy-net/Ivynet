use crate::{
    config::IvyConfig,
    docker::{dockercmd::DockerCmd, log::CmdLogSource},
    eigen::{contracts::delegation_manager::DelegationManager, quorum::QuorumType},
    error::IvyError,
    grpc::messages::NodeData,
    ivy_yaml::create_ivy_dockercompose,
    keychain::{KeyType, Keychain},
    messenger::BackendMessenger,
    rpc_management::{connect_provider, IvyProvider},
    utils::try_parse_chain,
    wallet::IvyWallet,
};
use async_trait::async_trait;
use config::AvsConfig;
use dialoguer::Select;
use ethers::{
    middleware::SignerMiddleware,
    providers::Middleware,
    signers::Signer,
    types::{Chain, H160, U256},
};
use lagrange::Lagrange;
use names::AvsName;
use semver::Version;
use std::{collections::HashMap, fmt::Debug, path::PathBuf, sync::Arc};
use tokio::process::Child;
use tracing::{debug, error, info};

pub mod commands;
pub mod config;
pub mod contracts;
pub mod eigenda;
pub mod error;
pub mod lagrange;
pub mod mach_avs;
pub mod names;

pub type QuorumMinMap = HashMap<Chain, HashMap<QuorumType, U256>>;

use self::{eigenda::EigenDA, mach_avs::AltLayer};

// TODO: Convenience functions on AVS type for display purposes, such as name()
// This could also implement Middleware.
#[allow(dead_code)] // TODO: use or remove registry coordinator
#[derive(Debug)]
pub struct AvsProvider {
    /// Signer and RPC provider
    pub provider: Arc<IvyProvider>,
    pub avs: Option<Box<dyn AvsVariant>>,
    // TODO: Deprecate this if possible, requires conversion of underlying AVS scripts
    pub keyfile_pw: Option<String>,
    pub delegation_manager: DelegationManager,
    pub messenger: Option<BackendMessenger>,
}

impl AvsProvider {
    pub fn new(
        avs: Option<Box<dyn AvsVariant>>,
        provider: Arc<IvyProvider>,
        keyfile_pw: Option<String>,
        messenger: Option<BackendMessenger>,
    ) -> Result<Self, IvyError> {
        let delegation_manager = DelegationManager::new(provider.clone())?;
        Ok(Self { avs, provider, keyfile_pw, delegation_manager, messenger })
    }

    /// Sets new avs with new provider
    pub async fn set_avs(
        &mut self,
        avs: Box<dyn AvsVariant>,
        provider: Arc<IvyProvider>,
    ) -> Result<(), IvyError> {
        self.with_avs(Some(avs)).await?;
        self.provider = provider;
        Ok(())
    }

    /// Replace the current AVS instance with a new instance.
    pub async fn with_avs(&mut self, avs: Option<Box<dyn AvsVariant>>) -> Result<(), IvyError> {
        self.avs = avs;
        Ok(())
    }

    /// Replace the current signer with a new signer.
    pub fn with_signer(&mut self, wallet: IvyWallet) -> Result<(), IvyError> {
        let provider = self.provider.provider().clone();
        let ivy_provider = SignerMiddleware::new(provider, wallet);
        self.provider = Arc::new(ivy_provider);
        Ok(())
    }

    pub fn with_keyfile_pw(&mut self, keyfile_pw: Option<String>) -> Result<(), IvyError> {
        self.keyfile_pw = keyfile_pw;
        Ok(())
    }

    /// Get a reference to the current runing AVS instance
    pub fn avs(&self) -> Result<&dyn AvsVariant, IvyError> {
        if let Some(avs) = &self.avs {
            Ok(&**avs)
        } else {
            Err(IvyError::AvsNotInitializedError)
        }
    }

    /// Get a mutable reference to the current runing AVS instance
    pub fn avs_mut(&mut self) -> Result<&mut Box<dyn AvsVariant>, IvyError> {
        if let Some(avs) = &mut self.avs {
            Ok(avs)
        } else {
            Err(IvyError::AvsNotInitializedError)
        }
    }

    /// Setup the loaded AVS instance. This includes both download and configuration steps.
    pub async fn setup(
        &mut self,
        config: &IvyConfig,
        operator_address: H160,
        bls_key_name: &str,
        bls_key_password: &str,
    ) -> Result<(), IvyError> {
        let provider = self.provider.clone();

        let setup_options = ["New Deployment", "Custom Attachment"];
        let setup_type = Select::new()
            .with_prompt(format!("Do you have an existing deployment of {}?", self.avs()?.name()))
            .items(&setup_options)
            .default(0)
            .interact()
            .unwrap();

        let is_custom = setup_type == 1;

        self.avs_mut()?
            .setup(provider, config, operator_address, bls_key_name, bls_key_password, is_custom)
            .await?;
        info!("Setup complete: run 'ivynet avs help' for next steps!");
        Ok(())
    }

    /// Start the loaded AVS instance. Returns an error if no AVS instance is loaded.
    pub async fn start(&mut self) -> Result<(), IvyError> {
        let avs_name = self.avs_mut()?.name();
        let is_running = self.avs_mut()?.is_running();
        let version = self.avs()?.version()?;
        let active_set = self.avs()?.active_set(self.provider.clone()).await;
        let signer = self.provider.signer().clone();
        if is_running {
            return Err(IvyError::AvsRunningError(
                avs_name.to_string(),
                Chain::try_from(signer.chain_id())?,
            ));
        }

        if let Some(messenger) = &mut self.messenger {
            let node_data = NodeData {
                operator_id: signer.address().as_bytes().to_vec(),
                avs_name: avs_name.to_string(),
                avs_version: version.to_string(),
                active_set,
            };
            messenger.send_node_data_payload(&node_data).await?;
        } else {
            println!("No messenger found - can't update data state");
        }

        self.avs_mut()?.start().await
    }

    pub async fn attach(&mut self) -> Result<Child, IvyError> {
        let avs_name = self.avs_mut()?.name();
        let is_running = self.avs_mut()?.is_running();
        let active_set = self.avs()?.active_set(self.provider.clone()).await;
        let version = self.avs()?.version()?;
        let signer = self.provider.signer().clone();
        if is_running {
            return Err(IvyError::AvsRunningError(
                avs_name.to_string(),
                Chain::try_from(signer.chain_id())?,
            ));
        }

        if let Some(messenger) = &mut self.messenger {
            let node_data = NodeData {
                operator_id: signer.address().as_bytes().to_vec(),
                avs_name: avs_name.to_string(),
                avs_version: version.to_string(),
                active_set,
            };
            messenger.send_node_data_payload(&node_data).await?;
        } else {
            println!("No messenger found - can't update data state");
        }
        self.avs_mut()?.attach().await
    }

    /// Stop the loaded AVS instance.
    pub async fn stop(&mut self) -> Result<(), IvyError> {
        let avs_name = self.avs_mut()?.name();
        let signer = self.provider.signer().clone();
        if let Some(messenger) = &mut self.messenger {
            messenger.delete_node_data_payload(signer.address(), avs_name).await?;
        } else {
            println!("No messenger found - can't update data state");
        }
        self.avs_mut()?.stop().await?;
        Ok(())
    }

    /// Clear the current AVS instance.
    pub async fn clear_avs(&mut self) -> Result<(), IvyError> {
        self.avs = None;
        Ok(())
    }

    pub async fn register(
        &self,
        operator_key_path: PathBuf,
        operator_key_pass: &str,
    ) -> Result<(), IvyError> {
        // TODO: Move quorum logic into AVS-specific implementations.
        // TODO: RIIA path creation? Move to new() func
        let avs_path = self.avs()?.base_path();
        std::fs::create_dir_all(avs_path.clone())?;

        // TODO: likely a function call in registry_coordinator
        // let status = DELEGATION_MANAGER.get_operator_status(self.client.address()).await?;
        // if status == 1 {
        //     //Check which quorums they're already in and register for the others they're eligible
        // for } else {
        //     //Register operator for all quorums they're eligible for
        // }

        self.avs()?
            .register(self.provider.clone(), avs_path.clone(), operator_key_path, operator_key_pass)
            .await?;

        Ok(())
    }

    pub async fn unregister(&self, _config: &IvyConfig) -> Result<(), IvyError> {
        let avs_path = self.avs()?.base_path();

        let keychain = Keychain::default();
        let keyname = keychain.select_key(KeyType::Ecdsa)?;
        let keypath = keychain.get_path(keyname);

        if let Some(pw) = &self.keyfile_pw {
            self.avs()?.unregister(self.provider.clone(), avs_path.clone(), keypath, pw).await?;
        } else {
            error!("No keyfile password provided. Exiting...");
            return Err(IvyError::KeyfilePasswordError);
        }

        Ok(())
    }

    pub async fn chain(&self) -> Result<Chain, IvyError> {
        Ok(Chain::try_from(self.provider.signer().chain_id())?)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait AvsVariant: Debug + Send + Sync + 'static {
    /// Perform all first-time setup steps for a given AVS instance. Includes an internal call to
    /// build_env
    async fn setup(
        &mut self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
        operator_address: H160,
        bls_key_name: &str,
        bls_key_password: &str,
        is_custom: bool,
    ) -> Result<(), IvyError>;

    //fn validate_install();
    fn validate_node_size(&self, quorum_percentage: U256) -> Result<bool, IvyError>;
    /// Register an operator for a given AVS. Implements AVS-specific logic.
    async fn register(
        &self,
        provider: Arc<IvyProvider>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        keyfile_password: &str,
    ) -> Result<(), IvyError>;
    /// Unregister an operator for a given AVS. Implements AVS-specific logic.
    async fn unregister(
        &self,
        provider: Arc<IvyProvider>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        keyfile_password: &str,
    ) -> Result<(), IvyError>;

    /// Start the AVS instance. Returns a Child process handle.
    async fn start(&mut self) -> Result<(), IvyError> {
        std::env::set_current_dir(self.run_path())?;
        debug!("docker start: {}", self.run_path().display());

        // Inject logging driver
        // TODO: fluentd address from env
        // This returns the name, which is just "ivy-docker-compose.yml." This can be stored or
        // just rely on the name of the string.
        let _ = create_ivy_dockercompose(
            self.run_path().join("docker-compose.yml"),
            "localhost:24224",
        )?;

        // NOTE: See the limitations of the Stdio::piped() method if this experiences a deadlock
        let cmd = DockerCmd::new()
            .args(["-f", "ivy-docker-compose.yml", "up", "--force-recreate"])
            .spawn()?;
        debug!("cmd PID: {:?}", cmd.id());
        self.set_running(true);
        Ok(())
    }

    /// Attach to the AVS instance. Returns a Child process handle.
    async fn attach(&mut self) -> Result<Child, IvyError> {
        //TODO: Better Pathing once invdividual configs are usable
        std::env::set_current_dir(self.run_path())?;
        let _ = create_ivy_dockercompose(
            self.run_path().join("docker-compose.yml"),
            "localhost:24224",
        )?;

        debug!("docker ataching: {}", self.run_path().display());
        // NOTE: See the limitations of the Stdio::piped() method if this experiences a deadlock
        let cmd = DockerCmd::new()
            .args(["-f", "ivy-docker-compose.yml", "up", "--force-recreate"])
            .spawn()?;

        debug!("cmd PID: {:?}", cmd.id());
        self.set_running(true);
        Ok(cmd)
    }

    /// Bring the AVS instance down.
    async fn stop(&mut self) -> Result<(), IvyError> {
        std::env::set_current_dir(self.run_path())?;
        // TODO: Deprecate env changing above

        // NOTE: See the limitations of the Stdio::piped() method if this experiences a deadlock
        let _ = DockerCmd::new().args(["-f", "ivy-docker-compose.yml", "down"]).status().await?;
        self.set_running(false);
        Ok(())
    }
    /// Handle a log line from the AVS instance.
    async fn handle_log(&self, line: &str, src: CmdLogSource) -> Result<(), IvyError>;
    /// Return the name of the AVS instance.
    fn name(&self) -> AvsName;
    /// Handle to the top-level directory for the AVS instance.
    fn base_path(&self) -> PathBuf;
    /// Return the path to the AVS instance's run directory (usually a docker compose file)
    fn run_path(&self) -> PathBuf;
    /// Return wether or not the AVS is running
    fn is_running(&self) -> bool;
    /// Set the running state of the AVS
    fn set_running(&mut self, running: bool);
    /// Get the version of the running avs
    fn version(&self) -> Result<Version, IvyError>;
    /// Get active set status of the running avs
    async fn active_set(&self, provider: Arc<IvyProvider>) -> bool;
}

// TODO: Builder pattern
pub async fn build_avs_provider(
    id: Option<&str>,
    chain: &str,
    config: &IvyConfig,
    wallet: Option<IvyWallet>,
    keyfile_pw: Option<String>,
    messenger: Option<BackendMessenger>,
) -> Result<AvsProvider, IvyError> {
    let chain = try_parse_chain(chain)?;
    let provider = connect_provider(&config.get_rpc_url(chain)?, wallet).await?;
    let avs_instance: Option<Box<dyn AvsVariant>> = if let Some(avs_id) = id {
        match AvsName::try_from(avs_id) {
            Ok(AvsName::EigenDA) => Some(Box::new(EigenDA::new_from_chain(chain))),
            Ok(AvsName::AltLayer) => Some(Box::new(AltLayer::new_from_chain(chain))),
            Ok(AvsName::LagrangeZK) => Some(Box::new(Lagrange::new_from_chain(chain))),
            _ => return Err(IvyError::InvalidAvsType(avs_id.to_string())),
        }
    } else {
        None
    };
    AvsProvider::new(avs_instance, Arc::new(provider), keyfile_pw, messenger)
}
