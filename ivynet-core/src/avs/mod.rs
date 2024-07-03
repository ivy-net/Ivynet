use crate::{
    config::IvyConfig,
    eigen::{
        delegation_manager::DelegationManager,
        quorum::{Quorum, QuorumType},
    },
    error::IvyError,
    rpc_management::{connect_provider, IvyProvider},
    utils::parse_chain,
    wallet::IvyWallet,
};
use async_trait::async_trait;
use ethers::{
    signers::Signer,
    types::{Address, Chain, U256},
};
use std::{collections::HashMap, fmt::Debug, fs, path::PathBuf, process::Child, sync::Arc};
use tracing::{error, info};

pub mod commands;
pub mod contracts;
pub mod eigenda;
pub mod instance;
pub mod mach_avs;

pub type QuorumMinMap = HashMap<Chain, HashMap<QuorumType, U256>>;

use self::{
    contracts::{RegistryCoordinator, RegistryCoordinatorAbi, StakeRegistry, StakeRegistryAbi},
    instance::AvsType,
};

// TODO: Convenience functions on AVS type for display purposes, such as name()
// This could also implement Middleware.
#[allow(dead_code)] // TODO: use or remove registry coordinator
#[derive(Debug)]
pub struct AvsProvider {
    /// Signer and RPC provider
    pub provider: Arc<IvyProvider>,
    pub avs: Option<AvsType>,
    // TODO: Deprecate this if possible, requires conversion of underlying AVS scripts
    pub keyfile_pw: Option<String>,
    stake_registry: Option<StakeRegistry>,
    registry_coordinator: Option<RegistryCoordinator>,
}

impl AvsProvider {
    pub fn new(avs: Option<AvsType>, provider: Arc<IvyProvider>, keyfile_pw: Option<String>) -> Self {
        let chain = Chain::try_from(provider.signer().chain_id()).unwrap_or_default();
        let (stake_registry, registry_coordinator) = if let Some(avs) = &avs {
            let stake_registry = StakeRegistryAbi::new(avs.stake_registry(chain), provider.clone());
            let registry_coordinator = RegistryCoordinatorAbi::new(avs.registry_coordinator(chain), provider.clone());
            (Some(stake_registry), Some(registry_coordinator))
        } else {
            (None, None)
        };
        Self { avs, provider, keyfile_pw, stake_registry, registry_coordinator }
    }

    /// Replace the current AVS instance with a new instance.
    pub async fn with_avs(&mut self, avs: Option<AvsType>) -> Result<(), IvyError> {
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let (stake_registry, registry_coordinator) = if let Some(avs) = &avs {
            let stake_registry = StakeRegistryAbi::new(avs.stake_registry(chain), self.provider.clone());
            let registry_coordinator =
                RegistryCoordinatorAbi::new(avs.registry_coordinator(chain), self.provider.clone());
            (Some(stake_registry), Some(registry_coordinator))
        } else {
            (None, None)
        };
        self.avs = avs;
        self.registry_coordinator = registry_coordinator;
        self.stake_registry = stake_registry;
        Ok(())
    }

    /// Get a reference to the current runing AVS instance
    pub fn avs(&self) -> Result<&AvsType, IvyError> {
        if let Some(avs) = &self.avs {
            Ok(avs)
        } else {
            Err(IvyError::AvsNotInitializedError)
        }
    }

    /// Get a mutable reference to the current runing AVS instance
    pub fn avs_mut(&mut self) -> Result<&mut AvsType, IvyError> {
        if let Some(avs) = &mut self.avs {
            Ok(avs)
        } else {
            Err(IvyError::AvsNotInitializedError)
        }
    }

    /// Get a reference to the current StakeRegistry contract for the loaded AVS.
    fn stake_registry(&self) -> Result<&StakeRegistry, IvyError> {
        if let Some(stake_registry) = &self.stake_registry {
            Ok(stake_registry)
        } else {
            Err(IvyError::AvsNotInitializedError)
        }
    }

    /// Get a reference to the current StakeRegistry contract for the loaded AVS.
    fn registry_coordinator(&self) -> Result<&RegistryCoordinator, IvyError> {
        if let Some(registry_coordinator) = &self.registry_coordinator {
            Ok(registry_coordinator)
        } else {
            Err(IvyError::AvsNotInitializedError)
        }
    }

    /// Setup the loaded AVS instance. This includes both download and configuration steps.
    pub async fn setup(&self, config: &IvyConfig) -> Result<(), IvyError> {
        self.avs()?.setup(self.provider.clone(), config).await?;
        info!("setup complete");
        Ok(())
    }

    /// Start the loaded AVS instance. Returns an error if no AVS instance is loaded.
    pub async fn start(&mut self) -> Result<Child, IvyError> {
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let quorums = self.get_bootable_quorums().await?;
        if quorums.is_empty() {
            error!("Could not launch EgenDA, no bootable quorums found. Exiting...");
            return Err(IvyError::NoQuorums);
        }
        self.avs_mut()?.start(quorums, chain).await
    }

    /// Stop the loaded AVS instance.
    pub async fn stop(&mut self, chain: Chain) -> Result<(), IvyError> {
        self.avs_mut()?.stop(chain).await?;
        Ok(())
    }

    /// Clear the current AVS instance.
    pub async fn clear_avs(&mut self) -> Result<(), IvyError> {
        self.avs = None;
        self.stake_registry = None;
        self.registry_coordinator = None;
        Ok(())
    }

    pub async fn opt_in(&self, config: &IvyConfig) -> Result<(), IvyError> {
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let quorums = self.get_bootable_quorums().await?;
        if quorums.is_empty() {
            error!("Could not launch EgenDA, no bootable quorums found. Exiting...");
            return Err(IvyError::NoQuorums);
        }

        let avs_path = self.avs()?.path();
        fs::create_dir_all(avs_path.clone())?;

        // TODO: likely a function call in registry_coordinator
        // let status = DELEGATION_MANAGER.get_operator_status(self.client.address()).await?;
        // if status == 1 {
        //     //Check which quorums they're already in and register for the others they're eligible for
        // } else {
        //     //Register operator for all quorums they're eligible for
        // }

        if let Some(pw) = &self.keyfile_pw {
            self.avs()?.opt_in(quorums, avs_path.clone(), config.default_private_keyfile.clone(), &pw, chain).await?;
        } else {
            error!("No keyfile password provided. Exiting...");
            return Err(IvyError::KeyfilePasswordError);
        }

        Ok(())
    }

    pub async fn opt_out(&self, config: &IvyConfig) -> Result<(), IvyError> {
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let quorums = self.get_bootable_quorums().await?;
        if quorums.is_empty() {
            error!("Could not launch EgenDA, no bootable quorums found. Exiting...");

            return Err(IvyError::NoQuorums);
        }

        let avs_path = self.avs()?.path();

        if let Some(pw) = &self.keyfile_pw {
            self.avs()?.opt_out(quorums, avs_path.clone(), config.default_private_keyfile.clone(), &pw, chain).await?;
        } else {
            error!("No keyfile password provided. Exiting...");
            return Err(IvyError::KeyfilePasswordError);
        }

        Ok(())
    }

    pub async fn get_bootable_quorums(&self) -> Result<Vec<QuorumType>, IvyError> {
        let mut quorums_to_boot: Vec<QuorumType> = Vec::new();
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let manager = DelegationManager::new(&self.provider);
        for quorum_type in self.avs()?.quorum_candidates(chain).iter() {
            let quorum = Quorum::try_from_type_and_network(*quorum_type, chain)?;
            let shares = manager.get_shares_for_quorum(self.provider.address(), &quorum).await?;
            let total_shares = shares.iter().fold(U256::from(0), |acc, x| acc + x); // This may be
                                                                                    // queryable from stake_registry or registry_coordinator directly?
            info!("Operator shares for quorum {}: {}", quorum_type, total_shares);
            let quorum_total = self.stake_registry()?.get_current_total_stake(*quorum_type as u8).await?;
            let quorum_percentage = total_shares * 10000 / (total_shares + quorum_total);
            if self.avs()?.validate_node_size(quorum_percentage)? {
                quorums_to_boot.push(*quorum_type);
            };
        }
        Ok(quorums_to_boot)
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
    async fn setup(&self, provider: Arc<IvyProvider>, config: &IvyConfig) -> Result<(), IvyError>;
    /// Builds the ENV file for the specific AVS + Chain combination. Writes changes to the local
    /// .env file. Check logs for specific file-paths.
    async fn build_env(&self, provider: Arc<IvyProvider>, config: &IvyConfig) -> Result<(), IvyError>;
    //fn validate_install();
    fn validate_node_size(&self, quorum_percentage: U256) -> Result<bool, IvyError>;
    async fn opt_in(
        &self,
        quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        keyfile_password: &str,
        chain: Chain,
    ) -> Result<(), IvyError>;
    async fn opt_out(
        &self,
        quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        keyfile_password: &str,
        chain: Chain,
    ) -> Result<(), IvyError>;
    async fn start(&mut self, quorums: Vec<QuorumType>, chain: Chain) -> Result<Child, IvyError>;
    async fn stop(&mut self, chain: Chain) -> Result<(), IvyError>;
    fn quorum_min(&self, chain: Chain, quorum_type: QuorumType) -> U256;
    fn quorum_candidates(&self, chain: Chain) -> Vec<QuorumType>;
    fn stake_registry(&self, chain: Chain) -> Address;
    fn registry_coordinator(&self, chain: Chain) -> Address;
    fn path(&self) -> PathBuf;
    /// Return wether or not the AVS is running
    fn running(&self) -> bool;
}

// TODO: Builder pattern
pub async fn build_avs_provider(
    id: Option<&str>,
    chain: &str,
    config: &IvyConfig,
    wallet: Option<IvyWallet>,
    keyfile_pw: Option<String>,
) -> Result<AvsProvider, IvyError> {
    let chain = parse_chain(chain);
    let provider = connect_provider(&config.get_rpc_url(chain)?, wallet).await?;
    let avs_instance = if let Some(avs_id) = id { Some(AvsType::new(avs_id, chain)?) } else { None };
    Ok(AvsProvider::new(avs_instance, Arc::new(provider), keyfile_pw))
}
