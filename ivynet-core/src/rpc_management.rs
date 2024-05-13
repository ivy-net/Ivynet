use ethers_core::k256;
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::Wallet;
use once_cell::sync::OnceCell;
use std::{convert::TryFrom, sync::Arc};

use crate::{config, keys};

pub type Client = Provider<Http>;
pub type Signer = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;

static NETWORK: OnceCell<Network> = OnceCell::new();
static PROVIDER: OnceCell<Provider<Http>> = OnceCell::new();
static SIGNER: OnceCell<Arc<Signer>> = OnceCell::new();
static CLIENT: OnceCell<Arc<Client>> = OnceCell::new();

#[derive(Debug, Clone, Copy)]
pub enum Network {
    Mainnet,
    Holesky,
    Local,
}

impl From<&str> for Network {
    fn from(network: &str) -> Network {
        match network {
            "mainnet" => Network::Mainnet,
            "testnet" => Network::Holesky,
            "holesky" => Network::Holesky,
            _ => Network::Local,
        }
    }
}

fn connect_provider() -> Provider<Http> {
    let cfg: config::IvyConfig = config::get_config();
    match get_network() {
        Network::Mainnet => {
            Provider::<Http>::try_from(cfg.mainnet_rpc_url.clone()).expect("Could not connect to provider")
        }
        Network::Holesky => {
            Provider::<Http>::try_from(cfg.testnet_rpc_url.clone()).expect("Could not connect to provider")
        }
        Network::Local => Provider::<Http>::try_from(cfg.local_rpc_url.clone()).expect("Could not connect to provider"),
    }
}

pub fn get_client() -> Arc<Client> {
    CLIENT.get_or_init(|| Arc::new(get_provider())).clone()
}

pub fn get_signer() -> Arc<Signer> {
    SIGNER.get_or_init(|| Arc::new(SignerMiddleware::new(get_provider(), keys::get_wallet()))).clone()
}

pub fn get_provider() -> Provider<Http> {
    PROVIDER.get_or_init(connect_provider).clone()
}

pub fn set_network(network: &str) -> Result<(), Network> {
    NETWORK.set(Network::from(network))
}

pub fn get_network() -> Network {
    *NETWORK.get_or_init(|| Network::Local)
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
