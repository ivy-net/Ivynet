use super::{
    dgm_info::{self, OperatorDetails},
    quorum::Quorum,
};

use crate::{
    error::IvyError,
    rpc_management::{self, IvyProvider},
};
use ethers::{
    signers::Signer,
    types::{Address, Chain, U256},
};
use tracing::info;

/// A global handle for the eigenlayer Delegation Manager contract:
/// https://github.com/Layr-Labs/eigenlayer-contracts/blob/testnet-holesky/src/contracts/core/DelegationManager.sol
pub struct DelegationManager(pub dgm_info::DelegationManagerAbi<rpc_management::IvyProvider>);

impl DelegationManager {
    pub fn new(provider: &IvyProvider) -> Self {
        let del_mgr_addr: Address =
            dgm_info::get_delegation_manager_address(Chain::try_from(provider.signer().chain_id()).unwrap_or_default());
        Self(dgm_info::DelegationManagerAbi::new(del_mgr_addr, provider.clone().into()))
    }

    pub async fn get_operator_details(&self, operator_address: Address) -> Result<(), IvyError> {
        let status = self.0.is_operator(operator_address).call().await?;
        println!("Is operator: {:?}", status);
        let details = self.0.operator_details(operator_address).call().await?;
        println!("Operator details: {:?}", details);

        Ok(())
    }

    pub async fn get_staker_delegatable_shares(&self, staker_address: Address) -> Result<(), IvyError> {
        let details = self.0.get_delegatable_shares(staker_address).await?;
        info!("Staker delegatable shares: {:?}", details);
        Ok(())
    }

    pub async fn get_shares_for_quorum(&self, operator: Address, quorum: &Quorum) -> Result<Vec<U256>, IvyError> {
        let strategies = quorum.0.iter().map(|strat| strat.address).collect();
        self.get_shares_for_strategies(operator, strategies).await
    }

    /// Function to get strategies' delegated stake to an operator
    pub async fn get_shares_for_strategies(
        &self,
        operator: Address,
        strategies: Vec<Address>,
    ) -> Result<Vec<U256>, IvyError> {
        info!("Shares for operator: {}", operator);
        let shares: Vec<U256> = self.0.get_operator_shares(operator, strategies).await?;
        Ok(shares)
    }

    pub async fn get_operator_status(&self, operator_address: Address) -> Result<bool, IvyError> {
        let status: bool = self.0.is_operator(operator_address).await?;
        println!("Operator status: {:?}", status);

        Ok(status)
    }

    pub async fn register(
        &self,
        earnings_receiver: Address,
        delegation_approver: Address,
        staker_opt_out_window_blocks: u32,
        metadata_uri: &str,
    ) -> Result<(), IvyError> {
        let operator_details = OperatorDetails {
            earnings_receiver, // Deprecated according to Eigenlayer docs
            delegation_approver,
            staker_opt_out_window_blocks,
        };
        self.0.register_as_operator(operator_details, metadata_uri.to_owned()).await?;
        Ok(())
    }
}
