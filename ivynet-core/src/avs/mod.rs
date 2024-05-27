pub mod avs_default;
pub mod contracts;
pub mod eigenda;
pub mod mach_avs;

use crate::{
    eigen::{
        delegation_manager::DELEGATION_MANAGER,
        quorum::{Quorum, QuorumType},
    },
    rpc_management::{Client, Network, Signer},
};
use dialoguer::Input;
use ethers_core::types::{Address, U256};
use std::{error::Error, fs, path::PathBuf, sync::Arc};
use tracing::{debug, error, info};

use self::contracts::{
    RegistryCoordinator, RegistryCoordinatorAbi, RegistryCoordinatorSigner, StakeRegistry, StakeRegistryAbi,
};

// TODO: Reduce cooridnator and coordinatorSigner to single field following condensed wallet/signer
// pattern
pub struct AvsProvider<T: AvsVariant> {
    avs: T,
    client: Arc<Signer>, // This can also be accessed by registry_coordinator_signer.client, only
    // reproduced here for clarity
    stake_registry: StakeRegistry,
    registry_coordinator: RegistryCoordinator,
    registry_coordinator_signer: RegistryCoordinatorSigner,
    env_path: PathBuf,
}

impl<T: AvsVariant> AvsProvider<T> {
    fn new(network: Network, avs: T, provider: Arc<Client>, signer: Arc<Signer>, env_path: PathBuf) -> Self {
        let stake_registry = StakeRegistryAbi::new(avs.stake_registry(network), provider.clone());
        let registry_coordinator = RegistryCoordinatorAbi::new(avs.registry_coordinator(network), provider);
        let registry_coordinator_signer =
            RegistryCoordinatorAbi::new(avs.registry_coordinator(network), signer.clone());
        Self { avs, client: signer, stake_registry, registry_coordinator, registry_coordinator_signer, env_path }
    }

    pub async fn boot(&self, network: Network) -> Result<(), Box<dyn Error>> {
        let quorums = self.get_bootable_quorums(network).await?;
        if quorums.is_empty() {
            error!("Could not launch EgenDA, no bootable quorums found. Exiting...");
            return Err("No bootable quorums found".into());
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
        self.avs.build_env(self.env_path.clone(), network).await?;
        self.avs.optin(quorums, network, self.env_path.clone()).await?;
        Ok(())
    }

    pub async fn get_bootable_quorums(&self, network: Network) -> Result<Vec<QuorumType>, Box<dyn std::error::Error>> {
        let mut quorums_to_boot: Vec<QuorumType> = Vec::new();
        for quorum_type in self.avs.quorum_candidates(network).iter() {
            let quorum = Quorum::try_from_type_and_network(*quorum_type, network)?;
            let shares = DELEGATION_MANAGER.get_shares_for_quorum(self.client.address(), &quorum).await?;
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

pub trait AvsVariant {
    async fn setup(&self, env_path: PathBuf) -> Result<(), Box<dyn Error>>;
    async fn build_env(&self, env_path: PathBuf, network: Network) -> Result<(), Box<dyn Error>>;
    //fn validate_install();
    fn validate_node_size(&self, quorum_percentage: U256, bandwidth: u32) -> Result<bool, Box<dyn std::error::Error>>;
    async fn optin(
        &self,
        quorums: Vec<QuorumType>,
        network: Network,
        eigen_path: PathBuf,
    ) -> Result<(), Box<dyn Error>>;
    fn quorum_min(&self, network: Network, quorum_type: QuorumType) -> U256;
    fn quorum_candidates(&self, network: Network) -> Vec<QuorumType>;
    fn stake_registry(&self, network: Network) -> Address;
    fn registry_coordinator(&self, network: Network) -> Address;
}
