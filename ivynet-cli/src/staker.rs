use clap::Parser;

use ethers::types::{Address, Chain};
use ivynet_core::{
    config::IvyConfig, eigen::delegation_manager::DelegationManager, rpc_management::connect_provider,
    wallet::IvyWallet,
};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum StakerCommands {
    #[command(name = "get-shares", about = "Get data on a staker's strategy choices and their stake in each one")]
    GetStakerShares { address: Address },
    #[command(name = "get-my-shares", about = "Get data on the saved keypair's current strategy and stake")]
    GetMyShares,
}

pub async fn parse_staker_subcommands(subcmd: StakerCommands, config: &IvyConfig, chain: Chain) -> Result<(), Error> {
    let provider = connect_provider(&config.get_rpc_url(chain)?, None)?;
    let manager = DelegationManager::new(&provider);
    match subcmd {
        StakerCommands::GetStakerShares { address } => manager.get_staker_delegatable_shares(address).await?,
        StakerCommands::GetMyShares => {
            let address = IvyWallet::address_from_file(config.default_public_keyfile.clone())?;
            manager.get_staker_delegatable_shares(address).await?
        }
    }
    Ok(())
}
