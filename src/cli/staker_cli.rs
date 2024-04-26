use clap::Parser;

use crate::{keys, eigen};

#[derive(Parser, Debug, Clone)]
pub(crate) enum StakerCommands {
    #[command(
        name = "get-shares",
        about = "Get data on a staker's strategy choices and their stake in each one"
    )]
    GetStakerShares {
        private_key: String,
    },
    #[command(
        name = "get-my-shares",
        about = "Get data on the saved keypair's current strategy and stake"
    )]
    GetMyShares,
}

pub async fn parse_staker_subcommands(subcmd: StakerCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        StakerCommands::GetStakerShares { private_key } => eigen::delegation_manager::get_staker_delegatable_shares(&private_key).await?,
        StakerCommands::GetMyShares => {
            let addr = keys::get_stored_public_key()?;
            eigen::delegation_manager::get_staker_delegatable_shares(&addr).await?
        },
    }
    Ok(())
}