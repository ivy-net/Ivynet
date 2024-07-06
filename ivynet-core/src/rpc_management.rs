use std::str::FromStr;

use ethers::{
    middleware::{signer::SignerMiddlewareError, SignerMiddleware},
    providers::{Http, Middleware, MiddlewareError, Provider},
    signers::Signer,
};

use crate::{error::IvyError, wallet::IvyWallet};

pub type IvyProvider = SignerMiddleware<Provider<Http>, IvyWallet>;

pub async fn connect_provider(rpc_url: &str, wallet: Option<IvyWallet>) -> Result<IvyProvider, IvyError> {
    let wallet = wallet.unwrap_or_default();
    let provider = Provider::new(Http::from_str(rpc_url)?);
    let chain = provider.get_chainid().await.map_err(|_| IvyError::UnknownNetwork)?;
    Ok(SignerMiddleware::new(provider, wallet.with_chain_id(chain.low_u64())))
}
