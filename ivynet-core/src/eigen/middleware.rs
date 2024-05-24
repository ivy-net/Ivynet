use ethers::types::Address;
use std::error::Error;

use crate::{
    avs::eigenda::eigenda_info::{self, RegistryCoordinator, StakeRegistry},
    rpc_management::IvyProvider,
};

/// EigenlayerMiddleware encapsulates functions common to eigenlayer registries and coordinators.
/// Methods implemented on this struct should exhibit identical behavior across deployments on
/// different networks (Mainnet, testnet, local, etc.)
pub struct EigenlayerMiddleware {
    stake_registry: StakeRegistry,
    registry_coordinator: RegistryCoordinator,
}

impl EigenlayerMiddleware {
    fn new(provider: &IvyProvider) -> Self {
        Self {
            stake_registry: eigenda_info::setup_stake_registry(provider),
            registry_coordinator: eigenda_info::setup_registry_coordinator(provider),
        }
    }

    pub async fn current_total_stake(&self, strategy_id: u8) -> Result<u128, Box<dyn Error>> {
        Ok(self.stake_registry.get_current_total_stake(strategy_id).await?)
    }
}

pub struct Operator {
    address: Address,
    eigen_middleware: EigenlayerMiddleware,
}

impl Operator {
    fn new(address: Address, provider: &IvyProvider) -> Self {
        Self { address, eigen_middleware: EigenlayerMiddleware::new(provider) }
    }
}
