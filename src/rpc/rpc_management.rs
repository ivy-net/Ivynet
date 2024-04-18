use ethers_core::k256;

use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::Wallet;
use std::{convert::TryFrom, sync::Mutex};
use std::sync::Arc;

use crate::{config, keys};

pub type Client = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

lazy_static::lazy_static! {
    pub static ref NETWORK: Mutex<String> = Mutex::new("".to_string());
    pub static ref PROVIDER: Provider<Http> = connect_provider();
    pub static ref CLIENT: Arc<Client> = Arc::new(SignerMiddleware::new(PROVIDER.clone(), keys::WALLET.clone()));
}

fn connect_provider() -> Provider<Http> {
    let cfg: config::IvyConfig = config::get_config();
    match *NETWORK.lock().unwrap() {
        ref network if network == "mainnet" => {
            Provider::<Http>::try_from(cfg.mainnet_rpc_url.clone()).expect("Could not connect to provider")
        }
        ref network if network == "testnet" => {
            Provider::<Http>::try_from(cfg.testnet_rpc_url.clone()).expect("Could not connect to provider")
        }
        _ => {
            Provider::<Http>::try_from(cfg.local_rpc_url.clone()).expect("Could not connect to provider")
        }
    }
}

pub fn set_network(network: String) {
    println!("Got here");
    *NETWORK.lock().unwrap() = network;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect_provider() {
        let provider = connect_provider();
        assert_eq!(provider.url().to_string(), "http://localhost:8545/");
    }
}
