use clap::Parser;

use ethers_core::types::Address;
use ivynet_core::{eigen::delegation_manager::DELEGATION_MANAGER, keys};

#[derive(Parser, Debug, Clone)]
pub(crate) enum StakerCommands {
    #[command(name = "get-shares", about = "Get data on a staker's strategy choices and their stake in each one")]
    GetStakerShares { address: Address },
    #[command(name = "get-my-shares", about = "Get data on the saved keypair's current strategy and stake")]
    GetMyShares,
}

pub async fn parse_staker_subcommands(subcmd: StakerCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        StakerCommands::GetStakerShares { address } => {
            DELEGATION_MANAGER.get_staker_delegatable_shares(address).await?
        }
        StakerCommands::GetMyShares => {
            let address = keys::get_stored_public_key()?;
            DELEGATION_MANAGER.get_staker_delegatable_shares(address).await?
        }
    }
    Ok(())
}
