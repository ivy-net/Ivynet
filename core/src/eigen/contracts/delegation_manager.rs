use std::{ops::Deref, sync::Arc};

use ethers::{
    contract::abigen,
    signers::Signer,
    types::{Address, Chain, H160},
};
use ivynet_macros::h160;

use crate::{
    eigen::strategy::{holesky::HOLESKY_LST_STRATEGIES, mainnet::MAINNET_LST_STRATEGIES},
    error::IvyError,
    rpc_management::IvyProvider,
};

abigen!(
    DelegationManagerAbi,
    "abi/DelegationManager.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

/// A delegation manager wrapper with constructor helpers and several utility methods. This struct
/// implements the `Deref` trait to allow for easy access to the underlying handle to the
/// delegation manager contract.
#[derive(Clone, Debug)]
pub struct DelegationManager {
    inner: DelegationManagerAbi<IvyProvider>,
    chain: Chain,
}

impl Deref for DelegationManager {
    type Target = DelegationManagerAbi<IvyProvider>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DelegationManager {
    pub fn new(provider: Arc<IvyProvider>) -> Result<Self, IvyError> {
        let chain: Chain = Chain::try_from(provider.signer().chain_id())?;
        let address = Self::chain_address(chain)?;
        let manager = Self { inner: DelegationManagerAbi::new(address, provider), chain };
        Ok(manager)
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
        self.register_as_operator(operator_details, metadata_uri.to_owned()).await?;
        Ok(())
    }

    /// Get the address of all eigenlayer strategies for active delgation manager contract.
    pub fn all_strategies(&self) -> Result<Vec<Address>, IvyError> {
        match self.chain {
            Chain::Mainnet => Ok(MAINNET_LST_STRATEGIES.iter().map(|s| s.address).collect()),
            Chain::Holesky => Ok(HOLESKY_LST_STRATEGIES.iter().map(|s| s.address).collect()),
            _ => Err(IvyError::ChainNotSupportedError(self.chain)),
        }
    }

    /// Get the address of the delegation manager contract for a given chain.
    pub fn chain_address(chain: Chain) -> Result<H160, IvyError> {
        match chain {
            Chain::Holesky => Ok(h160!(0xA44151489861Fe9e3055d95adC98FbD462B948e7)),
            Chain::Mainnet => Ok(h160!(0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A)),
            // May not be constant
            Chain::AnvilHardhat => Ok(h160!(0x30bdaE426d3CBD42e9d41D23958Fac6AD8310f81)),
            _ => Err(IvyError::ChainNotSupportedError(chain)),
        }
    }
}
