use clap::Parser;

use ivynet_core::{config::IvyConfig, ethers::types::Address};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum StakerCommands {
    #[command(name = "get", about = "Get data on a staker - defaults to local")]
    Get {
        #[command(subcommand)]
        subcmd: StakerGetterCommands,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum StakerGetterCommands {
    #[command(
        name = "shares",
        about = "Get data on a staker's strategy choices and their stake in each one <CHAIN> <<ADDRESS>>"
    )]
    GetStakerShares { chain: String, opt_address: Option<Address> },
}

pub async fn parse_staker_subcommands(
    subcmd: StakerCommands,
    config: &IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        StakerCommands::Get { subcmd } => {
            parse_staker_getter_subcommands(subcmd, config).await?;
        }
    }
    Ok(())
}

pub async fn parse_staker_getter_subcommands(
    subget: StakerGetterCommands,
    _config: &IvyConfig,
) -> Result<(), Error> {
    match subget {
        StakerGetterCommands::GetStakerShares { .. } => {
            todo!()
        }
    }
}
