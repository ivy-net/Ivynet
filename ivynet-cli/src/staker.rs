use clap::Parser;

use ivynet_core::{
    config::IvyConfig,
    eigen::delegation_manager::DelegationManager,
    ethers::types::{Address, Chain},
    rpc_management::connect_provider,
    wallet::IvyWallet,
};

use crate::{error::Error, utils::parse_chain};

#[derive(Parser, Debug, Clone)]
pub enum StakerCommands {
    #[command(
        name = "get-shares",
        about = "Get data on a staker's strategy choices and their stake in each one <CHAIN> <ADDRESS>"
    )]
    GetStakerShares { chain: String, address: Address },
    #[command(name = "get-my-shares", about = "Get data on the saved keypair's current strategy and stake <CHAIN>")]
    GetMyShares { chain: String },
}

pub async fn parse_staker_subcommands(subcmd: StakerCommands, config: &IvyConfig) -> Result<(), Error> {
    match subcmd {
        StakerCommands::GetStakerShares { address, chain } => {
            let chain = parse_chain(&chain);
            let provider = connect_provider(&config.get_rpc_url(chain)?, None).await?;
            let manager = DelegationManager::new(&provider);
            manager.get_staker_delegatable_shares(address).await?
        }
        StakerCommands::GetMyShares { chain } => {
            let chain = parse_chain(&chain);
            let provider = connect_provider(&config.get_rpc_url(chain)?, None).await?;
            let manager = DelegationManager::new(&provider);
            let address = IvyWallet::address_from_file(config.default_public_keyfile.clone())?;
            manager.get_staker_delegatable_shares(address).await?;
        }
    }
    Ok(())
}
