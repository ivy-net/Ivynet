use clap::Parser;

use dialoguer::Password;
use ivynet_core::{
    config::IvyConfig,
    ethers::types::Chain,
    server::{handle_avs_command, AvsHandleCommands},
    wallet::IvyWallet,
};
use tracing::debug;

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
    // TODO: Remove this prompt from library
    // Not every AVS instance requires access to a wallet. How best to handle this? Enum variant?
    let password: String = Password::new().with_prompt("Input the password for your stored keyfile").interact()?;
    let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), password)?;
    match subcmd {
        AvsCommands::Setup { avs } => {
            handle_avs_command(AvsHandleCommands::Setup, &avs, config, chain, Some(wallet)).await?
        }
        AvsCommands::Optin { avs } => {
            handle_avs_command(AvsHandleCommands::Optin, &avs, config, chain, Some(wallet)).await?
        }
        AvsCommands::Optout { avs } => {
            handle_avs_command(AvsHandleCommands::Optout, &avs, config, chain, Some(wallet)).await?
        }
        AvsCommands::Start { avs } => {
            handle_avs_command(AvsHandleCommands::Start, &avs, config, chain, Some(wallet)).await?
        }
        AvsCommands::Stop { avs } => {
            handle_avs_command(AvsHandleCommands::Start, &avs, config, chain, Some(wallet)).await?
        }
        _ => todo!(),
    };
    Ok(())
}
