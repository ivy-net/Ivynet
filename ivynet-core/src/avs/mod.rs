use crate::{
    config::IvyConfig,
    eigen::{
        delegation_manager::DelegationManager,
        quorum::{Quorum, QuorumType},
    },
    error::IvyError,
    rpc_management::IvyProvider,
};
use async_trait::async_trait;
use ethers::{
    signers::Signer,
    types::{Address, Chain, U256},
};
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};
use tracing::{error, info};

pub mod contracts;
pub mod eigenda;
pub mod instance;
pub mod mach_avs;

pub type QuorumMinMap = HashMap<Chain, HashMap<QuorumType, U256>>;

use self::contracts::{RegistryCoordinator, RegistryCoordinatorAbi, StakeRegistry, StakeRegistryAbi};

#[allow(dead_code)] // TODO: use or remove registry coordinator
pub struct AvsProvider<T: AvsVariant> {
    avs: T,
    provider: Arc<IvyProvider>,
    stake_registry: StakeRegistry,
    registry_coordinator: RegistryCoordinator,
}

impl<T: AvsVariant> AvsProvider<T> {
    pub fn new(chain: Chain, avs: T, provider: Arc<IvyProvider>) -> Self {
        let stake_registry = StakeRegistryAbi::new(avs.stake_registry(chain), provider.clone());
        let registry_coordinator = RegistryCoordinatorAbi::new(avs.registry_coordinator(chain), provider.clone());
        Self { avs, provider, stake_registry, registry_coordinator }
    }

    pub async fn setup(&self, config: &IvyConfig) -> Result<(), IvyError> {
        self.avs.setup(self.provider.clone(), config).await?;
        info!("setup complete");
        Ok(())
    }

    pub async fn start(&self, config: &IvyConfig) -> Result<(), IvyError> {
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let quorums = self.get_bootable_quorums().await?;
        if quorums.is_empty() {
            error!("Could not launch EgenDA, no bootable quorums found. Exiting...");
            return Err(IvyError::NoQuorums);
        }

        self.avs.start(quorums, chain).await
    }

    pub async fn stop(&self, config: &IvyConfig) -> Result<(), IvyError> {
        todo!();
    }

    pub async fn opt_in(&self, config: &IvyConfig) -> Result<(), IvyError> {
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let quorums = self.get_bootable_quorums().await?;
        if quorums.is_empty() {
            error!("Could not launch EgenDA, no bootable quorums found. Exiting...");

            return Err(IvyError::NoQuorums);
        }

        let avs_path = self.avs.path();

        fs::create_dir_all(avs_path.clone())?;

        // TODO: likely a function call in registry_coordinator
        // let status = DELEGATION_MANAGER.get_operator_status(self.client.address()).await?;
        // if status == 1 {
        //     //Check which quorums they're already in and register for the others they're eligible for
        // } else {
        //     //Register operator for all quorums they're eligible for
        // }

        self.avs.opt_in(quorums, avs_path.clone(), config.default_private_keyfile.clone(), chain).await?;
        Ok(())
    }

    pub async fn opt_out(&self, config: &IvyConfig) -> Result<(), IvyError> {
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let quorums = self.get_bootable_quorums().await?;
        if quorums.is_empty() {
            error!("Could not launch EgenDA, no bootable quorums found. Exiting...");

            return Err(IvyError::NoQuorums);
        }

        let avs_path = self.avs.path();

        self.avs.opt_out(quorums, avs_path.clone(), config.default_private_keyfile.clone(), chain).await?;
        Ok(())
    }

    pub async fn get_bootable_quorums(&self) -> Result<Vec<QuorumType>, IvyError> {
        let mut quorums_to_boot: Vec<QuorumType> = Vec::new();
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let manager = DelegationManager::new(&self.provider);
        for quorum_type in self.avs.quorum_candidates(chain).iter() {
            let quorum = Quorum::try_from_type_and_network(*quorum_type, chain)?;
            let shares = manager.get_shares_for_quorum(self.provider.address(), &quorum).await?;
            let total_shares = shares.iter().fold(U256::from(0), |acc, x| acc + x); // This may be
                                                                                    // queryable from stake_registry or registry_coordinator directly?
            info!("Operator shares for quorum {}: {}", quorum_type, total_shares);
            let quorum_total = self.stake_registry.get_current_total_stake(*quorum_type as u8).await?;
            let quorum_percentage = total_shares * 10000 / (total_shares + quorum_total);
            if self.avs.validate_node_size(quorum_percentage)? {
                quorums_to_boot.push(*quorum_type);
            };
        }
        Ok(quorums_to_boot)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait AvsVariant {
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
        chain: Chain,
    ) -> Result<(), IvyError>;
    async fn opt_out(
        &self,
        quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        chain: Chain,
    ) -> Result<(), IvyError>;
    async fn start(&self, quorums: Vec<QuorumType>, chain: Chain) -> Result<(), IvyError>;
    async fn stop(&self, quorums: Vec<QuorumType>, chain: Chain) -> Result<(), IvyError>;
    fn quorum_min(&self, chain: Chain, quorum_type: QuorumType) -> U256;
    fn quorum_candidates(&self, chain: Chain) -> Vec<QuorumType>;
    fn stake_registry(&self, chain: Chain) -> Address;
    fn registry_coordinator(&self, chain: Chain) -> Address;
    fn path(&self) -> PathBuf;
}
