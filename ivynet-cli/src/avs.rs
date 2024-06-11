use clap::Parser;

use ethers::types::Chain;
use ivynet_core::{
    config::IvyConfig,
    server::{handle_avs_command, AvsHandleCommands},
};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum AvsCommands {
    #[command(name = "setup", about = "opt in to valid quorums with the given AVS")]
    Setup { avs: String },
    #[command(name = "optin", about = "opt in to valid quorums with the given AVS")]
    Optin { avs: String },
    #[command(name = "optout", about = "opt out of valid quorums with the given AVS")]
    Optout { avs: String },
    #[command(name = "start", about = "Start running an AVS node in a docker container")]
    Start { avs: String },
    #[command(name = "stop", about = "stop running the active AVS docker container")]
    Stop { avs: String },
    #[command(
        name = "check-stake-percentage",
        about = "Determine what percentage of the total stake an address would have"
    )]
    CheckStakePercentage { avs: String, address: String, network: String },
}

pub async fn parse_config_subcommands(subcmd: AvsCommands, config: &IvyConfig, chain: Chain) -> Result<(), Error> {
    // TODO! We need to decrypt wallet here FIRST
    match subcmd {
        AvsCommands::Setup { avs } => handle_avs_command(AvsHandleCommands::Setup, &avs, config, chain, None).await?,
        AvsCommands::Optin { avs } => handle_avs_command(AvsHandleCommands::Optin, &avs, config, chain, None).await?,
        AvsCommands::Optout { avs } => handle_avs_command(AvsHandleCommands::Optout, &avs, config, chain, None).await?,
        AvsCommands::Start { avs } => handle_avs_command(AvsHandleCommands::Start, &avs, config, chain, None).await?,
        AvsCommands::Stop { avs } => handle_avs_command(AvsHandleCommands::Start, &avs, config, chain, None).await?,
        _ => todo!(),
    };
    Ok(())
}
