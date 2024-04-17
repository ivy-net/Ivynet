use ethers_core::k256;

use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::Wallet;
use std::convert::TryFrom;
use std::sync::Arc;

use crate::{config, keys};

pub type Client = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

lazy_static::lazy_static! {
    pub static ref PROVIDER: Provider<Http> = connect_provider();
    pub static ref CLIENT: Arc<Client> = Arc::new(SignerMiddleware::new(PROVIDER.clone(), keys::WALLET.clone()));
}

fn connect_provider() -> Provider<Http> {
    let cfg: config::IvyConfig = config::get_config();
    Provider::<Http>::try_from(&cfg.local_rpc_url).expect("Could not connect to provider")
}
