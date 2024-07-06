use std::sync::Arc;

use clap::Parser;

use ivynet_core::{
    config::IvyConfig,
    eigen::contracts::delegation_manager::DelegationManager,
    ethers::types::Address,
    rpc_management::connect_provider,
    utils::{parse_chain, unwrap_or_local},
};

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

pub async fn parse_staker_subcommands(subcmd: StakerCommands, config: &IvyConfig) -> Result<(), Error> {
    match subcmd {
        StakerCommands::Get { subcmd } => {
            parse_staker_getter_subcommands(subcmd, config).await?;
        }
    }
    Ok(())
}

pub async fn parse_staker_getter_subcommands(subget: StakerGetterCommands, config: &IvyConfig) -> Result<(), Error> {
    match subget {
        StakerGetterCommands::GetStakerShares { opt_address, chain } => {
            let chain = parse_chain(&chain);
            let provider = connect_provider(&config.get_rpc_url(chain)?, None).await?;
            let manager = DelegationManager::new(Arc::new(provider))?;
            todo!()
        }
    }
    Ok(())
}
