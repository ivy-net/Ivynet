use std::str::FromStr;

use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Provider},
};

use crate::{error::IvyError, wallet::IvyWallet};

pub type IvyProvider = SignerMiddleware<Provider<Http>, IvyWallet>;

pub fn connect_provider(rpc_url: &str, wallet: Option<IvyWallet>) -> Result<IvyProvider, IvyError> {
    let wallet = wallet.unwrap_or_default();
    Ok(SignerMiddleware::new(Provider::new(Http::from_str(rpc_url)?), wallet))
}
