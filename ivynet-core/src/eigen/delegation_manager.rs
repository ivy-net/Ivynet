use super::{
    dgm_info,
    strategy::{EigenStrategy, StrategyList},
};
use crate::rpc_management;
use ethers_core::types::{Address, U256};
use once_cell::sync::Lazy;
use std::collections::HashMap;
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
        println!("Staker delegatable shares: {:?}", details);
        Ok(())
    }

    /// Function to get strategies' delegated stake to an operator
    /// TODO: This currently grabs strategies directly from the type, which may not be the best
    /// mode of iteraction.
    pub async fn get_shares_for_strategies<T: StrategyList<T> + EigenStrategy>(
        &self,
        operator_address: Address,
    ) -> Result<HashMap<T, U256>, Box<dyn std::error::Error>> {
        info!("Shares for operator: {:?}", operator_address);
        let strategies = T::get_all();
        let strategy_addresses: Vec<Address> = strategies.iter().map(|strat| strat.address()).collect();
        let shares: Vec<U256> = self.0.get_operator_shares(operator_address, strategy_addresses).call().await?;

        let mut stake_map: HashMap<T, U256> = HashMap::new();
        for (i, strat) in strategies.iter().enumerate() {
            stake_map.insert(*strat, shares[i]);
        }

        Ok(stake_map)
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
