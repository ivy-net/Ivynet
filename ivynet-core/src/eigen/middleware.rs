use ethers_core::types::Address;
use std::error::Error;

use crate::avs::eigenda::eigenda_info::{self, RegistryCoordinator, RegistryCoordinatorSigner, StakeRegistry};

/// EigenlayerMiddleware encapsulates functions common to eigenlayer registries and coordinators.
/// Methods implemented on this struct should exhibit identical behavior across deployments on
/// different networks (Mainnet, testnet, local, etc.)
pub struct EigenlayerMiddleware {
    // TODO: Determine which of these should be per-AVS.
    stake_registry: StakeRegistry,
    registry_coordinatior: RegistryCoordinator,
    registry_signer: RegistryCoordinatorSigner,
}

impl EigenlayerMiddleware {
    fn new() -> Self {
        Self {
            stake_registry: eigenda_info::setup_stake_registry(),
            registry_coordinatior: eigenda_info::setup_registry_coordinator(),
            registry_signer: eigenda_info::setup_registry_coordinator_signer(),
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
    fn new(address: Address) -> Self {
        Self { address, eigen_middleware: EigenlayerMiddleware::new() }
    }
}
