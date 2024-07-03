use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::Signer,
};
use std::str::FromStr;

use crate::{
    eigen::delegation_manager::DelegationManager, error::IvyError, rpc_management::IvyProvider, wallet::IvyWallet,
};

pub struct Operator {
    provider: IvyProvider,
    delegation_manager: DelegationManager,
}

impl Operator {
    pub async fn new(provider: IvyProvider, wallet: Option<IvyWallet>) -> Result<Self, IvyError> {
        let wallet = wallet.unwrap_or_default();
        let delegation_manager = DelegationManager::new(&provider);
        Ok(Operator { provider, delegation_manager })
    }
}
