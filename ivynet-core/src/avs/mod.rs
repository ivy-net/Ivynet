use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

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
use dialoguer::Input;
use ethers::{
    signers::Signer,
    types::{Address, Chain, U256},
};
use tracing::{error, info};

pub mod avs_default;
pub mod contracts;
pub mod eigenda;
pub mod mach_avs;

pub type QuorumMinMap = HashMap<Chain, HashMap<QuorumType, U256>>;

use self::contracts::{RegistryCoordinator, RegistryCoordinatorAbi, StakeRegistry, StakeRegistryAbi};

#[allow(dead_code)] // TODO: use or remove registry coordinator
pub struct AvsProvider<T: AvsVariant> {
    avs: T,
    provider: Arc<IvyProvider>,
    stake_registry: StakeRegistry,
    registry_coordinator: RegistryCoordinator,
    env_path: PathBuf,
}

impl<T: AvsVariant> AvsProvider<T> {
    fn new(chain: Chain, avs: T, provider: Arc<IvyProvider>, env_path: PathBuf) -> Self {
        let stake_registry = StakeRegistryAbi::new(avs.stake_registry(chain), provider.clone());
        let registry_coordinator = RegistryCoordinatorAbi::new(avs.registry_coordinator(chain), provider.clone());
        Self { avs, provider, stake_registry, registry_coordinator, env_path }
    }

    pub async fn boot(&self, config: &IvyConfig) -> Result<(), IvyError> {
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let quorums = self.get_bootable_quorums().await?;
        if quorums.is_empty() {
            error!("Could not launch EgenDA, no bootable quorums found. Exiting...");

            return Err(IvyError::NoQuorums);
        }

        fs::create_dir_all(&self.env_path)?;

        // TODO: likely a function call in registry_coordinator
        // let status = DELEGATION_MANAGER.get_operator_status(self.client.address()).await?;
        // if status == 1 {
        //     //Check which quorums they're already in and register for the others they're eligible for
        // } else {
        //     //Register operator for all quorums they're eligible for
        // }

        self.avs.setup(self.env_path.clone()).await?;
        self.avs.build_env(self.env_path.clone(), self.provider.clone(), config).await?;
        self.avs.optin(quorums, self.env_path.clone(), config.default_private_keyfile.clone(), chain).await?;
        Ok(())
    }

    pub async fn get_bootable_quorums(&self) -> Result<Vec<QuorumType>, IvyError> {
        let mut quorums_to_boot: Vec<QuorumType> = Vec::new();
        let chain = Chain::try_from(self.provider.signer().chain_id()).unwrap_or_default();
        let manager = DelegationManager::new(&*self.provider);
        for quorum_type in self.avs.quorum_candidates(chain).iter() {
            let quorum = Quorum::try_from_type_and_network(*quorum_type, chain)?;
            let shares = manager.get_shares_for_quorum(self.provider.address(), &quorum).await?;
            let total_shares = shares.iter().fold(U256::from(0), |acc, x| acc + x); // This may be
                                                                                    // queryable from stake_registry or registry_coordinator directly?
            info!("Operator shares for quorum {}: {}", quorum_type, total_shares);
            let quorum_total = self.stake_registry.get_current_total_stake(*quorum_type as u8).await?;
            let quorum_percentage = total_shares * 10000 / (total_shares + quorum_total);
            let bandwidth: u32 = Input::new().with_prompt("Input your bandwidth in mbps").interact_text()?;
            if self.avs.validate_node_size(quorum_percentage, bandwidth)? {
                quorums_to_boot.push(*quorum_type);
            };
        }
        Ok(quorums_to_boot)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait AvsVariant {
    async fn setup(&self, env_path: PathBuf) -> Result<(), IvyError>;
    async fn build_env(
        &self,
        env_path: PathBuf,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
    ) -> Result<(), IvyError>;
    //fn validate_install();
    fn validate_node_size(&self, quorum_percentage: U256, bandwidth: u32) -> Result<bool, IvyError>;
    async fn optin(
        &self,
        quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        chain: Chain,
    ) -> Result<(), IvyError>;
    fn quorum_min(&self, chain: Chain, quorum_type: QuorumType) -> U256;
    fn quorum_candidates(&self, chain: Chain) -> Vec<QuorumType>;
    fn stake_registry(&self, chain: Chain) -> Address;
    fn registry_coordinator(&self, chain: Chain) -> Address;
}
