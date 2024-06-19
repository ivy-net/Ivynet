use ivynet_core::{
    config::IvyConfig,
    error::IvyError,
    ethers::types::{Address, Chain},
    wallet::IvyWallet,
};
use tracing::warn;

pub fn parse_chain(chain: &str) -> Chain {
    chain.parse::<Chain>().unwrap_or_else(|_| {
        warn!("unknown network: {chain}, defaulting to anvil_hardhat at 31337");
        Chain::AnvilHardhat
    })
}
pub fn unwrap_or_local(opt_address: Option<Address>, config: IvyConfig) -> Result<Address, IvyError> {
    match opt_address {
        Some(address) => Ok(address),
        None => {
            warn!("No address provided, defaulting to local wallet address");
            IvyWallet::address_from_file(config.default_public_keyfile.clone())
        }
    }
}