use ethers::types::Chain;

use crate::error::IvyError;

pub fn try_parse_chain(chain: &str) -> Result<Chain, IvyError> {
    chain.parse::<Chain>().map_err(|_| IvyError::ChainParseError(chain.to_owned()))
}

pub fn gb_to_bytes(gb: u64) -> u64 {
    gb * 10u64.pow(9)
}
