use super::{dgm_info, quorum::Quorum};
use crate::rpc_management;
use ethers_core::types::{Address, U256};
use once_cell::sync::Lazy;
use std::error::Error;
use tracing::info;

/// A global handle for the eigenlayer Delegation Manager contract:
/// https://github.com/Layr-Labs/eigenlayer-contracts/blob/testnet-holesky/src/contracts/core/DelegationManager.sol
///
pub static DELEGATION_MANAGER: Lazy<DelegationManager> = Lazy::new(DelegationManager::new);

pub struct DelegationManager(pub dgm_info::DelegationManagerAbi<rpc_management::Client>);

impl DelegationManager {
    pub fn new() -> Self {
        let del_mgr_addr: Address =
            dgm_info::get_delegation_manager_address().parse().expect("Could not parse DelegationManager address");
        Self(dgm_info::DelegationManagerAbi::new(del_mgr_addr, rpc_management::get_client()))
    }

    pub async fn get_operator_details(&self, operator_address: Address) -> Result<(), Box<dyn std::error::Error>> {
        let status = self.0.is_operator(operator_address).call().await?;
        println!("Is operator: {:?}", status);
        let details = self.0.operator_details(operator_address).call().await?;
        println!("Operator details: {:?}", details);

        Ok(())
    }

    pub async fn get_staker_delegatable_shares(
        &self,
        staker_address: Address,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let details = self.0.get_delegatable_shares(staker_address).call().await?;
        info!("Staker delegatable shares: {:?}", details);
        Ok(())
    }

    pub async fn get_shares_for_quorum(&self, operator: Address, quorum: &Quorum) -> Result<Vec<U256>, Box<dyn Error>> {
        let strategies = quorum.0.iter().map(|strat| strat.address).collect();
        Ok(self.get_shares_for_strategies(operator, strategies).await?)
    }

    /// Function to get strategies' delegated stake to an operator
    pub async fn get_shares_for_strategies(
        &self,
        operator: Address,
        strategies: Vec<Address>,
    ) -> Result<Vec<U256>, Box<dyn std::error::Error>> {
        info!("Shares for operator: {}", operator);
        let shares: Vec<U256> = self.0.get_operator_shares(operator, strategies).call().await?;
        Ok(shares)
    }

    pub async fn get_operator_status(&self, operator_address: Address) -> Result<bool, Box<dyn std::error::Error>> {
        let status: bool = self.0.is_operator(operator_address).call().await?;
        println!("Operator status: {:?}", status);

        Ok(status)
    }
}

impl Default for DelegationManager {
    fn default() -> Self {
        Self::new()
    }
}
