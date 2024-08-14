use ethers::types::Chain;
use tracing::warn;

use crate::{config::IvyConfig, error::IvyError, ethers::types::Address};

pub fn try_parse_chain(chain: &str) -> Result<Chain, IvyError> {
    chain.parse::<Chain>().map_err(|_| IvyError::ChainParseError(chain.to_owned()))
}

pub fn unwrap_or_local(
    opt_address: Option<Address>,
    config: IvyConfig,
) -> Result<Address, IvyError> {
    match opt_address {
        Some(address) => Ok(address),
        None => {
            warn!("No address provided, defaulting to local wallet address");
            Ok(config.default_ecdsa_address)
        }
    }
}

pub fn gb_to_bytes(gb: u64) -> u64 {
    gb * 10u64.pow(9)
}
