use ethers_core::k256;

use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::Wallet;
use std::convert::TryFrom;
use std::sync::Arc;

use crate::{config, keys};

pub type Client = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

lazy_static::lazy_static! {
    pub static ref NETWORK: String = ;
    pub static ref PROVIDER: Provider<Http> = connect_provider();
    pub static ref CLIENT: Arc<Client> = Arc::new(SignerMiddleware::new(PROVIDER.clone(), keys::WALLET.clone()));
}

fn connect_provider() -> Provider<Http> {
    
    let cfg: config::IvyConfig = todo!(); //config::get_config();
    Provider::<Http>::try_from(&NETWORK).expect("Could not connect to provider")
}

//TODO: Need to build out RPC functionality properly

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect_provider() {
        let provider = connect_provider();
        assert_eq!(provider.url().to_string(), "http://localhost:8545");
    }
}
