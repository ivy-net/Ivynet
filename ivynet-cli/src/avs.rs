use std::fmt::Display;

use clap::Subcommand;
use dialoguer::Password;
use ivynet_core::{
    config::IvyConfig,
    server::{handle_avs_command, AvsHandleCommands},
    wallet::IvyWallet,
};

#[allow(unused_imports)]
use tracing::{info, warn};

use crate::{error::Error, utils::parse_chain};

#[derive(Subcommand, Debug)]
pub enum AvsCommands {
    #[command(name = "setup", about = "Set up environment and download required files for an AVS <CHAIN> <AVS>")]
    Setup { avs: String, chain: String },
    #[command(name = "optin", about = "Opt in to valid quorums with the given AVS <CHAIN> <AVS>")]
    Optin { avs: String, chain: String },
    #[command(name = "optout", about = "Opt out of valid quorums with the given AVS <CHAIN> <AVS>")]
    Optout { avs: String, chain: String },
    #[command(name = "start", about = "Start running an AVS node in a docker container <CHAIN> <AVS>")]
    Start { avs: String, chain: String },
    #[command(name = "stop", about = "Stop running the active AVS docker container <CHAIN> <AVS>")]
    Stop { avs: String, chain: String },
    #[command(
        name = "check-stake-percentage",
        about = "Determine what percentage of the total stake an address would have <AVS> <ADDRESS> <CHAIN>"
    )]
    CheckStakePercentage { _avs: String, address: String, chain: String },
}

impl Display for AvsCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AvsCommands::Setup { avs, chain } => write!(f, "setup {} on chain {}", avs, chain),
            AvsCommands::Optin { avs, chain } => write!(f, "optin {} on chain {}", avs, chain),
            AvsCommands::Optout { avs, chain } => write!(f, "optout {} on chain {}", avs, chain),
            AvsCommands::Start { avs, chain } => write!(f, "start {} on chain {}", avs, chain),
            AvsCommands::Stop { avs, chain } => write!(f, "stop {} on chain {}", avs, chain),
            AvsCommands::CheckStakePercentage { _avs, address, chain } => {
                write!(f, "check stake percentage for {} on {} network", address, chain)
            }
        }
    }
}

pub async fn parse_avs_subcommands(subcmd: AvsCommands, config: &IvyConfig) -> Result<(), Error> {
    // Not every AVS instance requires access to a wallet. How best to handle this? Enum variant?
    let password: String =
        Password::new().with_prompt("Input the password for your stored ECDSA keyfile").interact()?;
    let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), password)?;
    info!("Avs Command: {subcmd}");
    match subcmd {
        AvsCommands::Setup { avs, chain } => {
            let chain = parse_chain(&chain);
            handle_avs_command(AvsHandleCommands::Setup, &avs, config, chain, Some(wallet)).await?
        }
        AvsCommands::Optin { avs, chain } => {
            let chain = parse_chain(&chain);
            handle_avs_command(AvsHandleCommands::Optin, &avs, config, chain, Some(wallet)).await?
        }
        AvsCommands::Optout { avs, chain } => {
            let chain = parse_chain(&chain);
            handle_avs_command(AvsHandleCommands::Optout, &avs, config, chain, Some(wallet)).await?
        }
        AvsCommands::Start { avs, chain } => {
            let chain = parse_chain(&chain);
            handle_avs_command(AvsHandleCommands::Start, &avs, config, chain, Some(wallet)).await?
        }
        AvsCommands::Stop { avs, chain } => {
            let chain = parse_chain(&chain);
            handle_avs_command(AvsHandleCommands::Start, &avs, config, chain, Some(wallet)).await?
        }
        _ => todo!(),
    };
    Ok(())
}
