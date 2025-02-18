use ethers::types::Chain;

use crate::error::DatabaseError;

pub fn try_parse_chain(chain: &str) -> Result<Chain, DatabaseError> {
    chain.parse::<Chain>().map_err(|_| DatabaseError::ChainParseError(chain.to_owned()))
}

pub fn gb_to_bytes(gb: u64) -> u64 {
    gb * 10u64.pow(9)
}
