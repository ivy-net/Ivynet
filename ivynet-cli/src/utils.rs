use ivynet_core::ethers::types::Chain;
use tracing::warn;

pub fn parse_chain(chain: &str) -> Chain {
    chain.parse::<Chain>().unwrap_or_else(|_| {
        warn!("unknown network: {chain}, defaulting to anvil_hardhat at 31337");
        Chain::AnvilHardhat
    })
}
