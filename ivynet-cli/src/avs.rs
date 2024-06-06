use clap::Parser;

use ethers::types::Chain;
use ivynet_core::{avs, config::IvyConfig};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum AvsCommands {
    #[command(name = "optin", about = "optin to valid quorums with the given AVS")]
    Optin { avs: String },
    #[command(
        name = "check-stake-percentage",
        about = "Determine what percentage of the total stake an address would have"
    )]
    CheckStakePercentage { avs: String, address: String, network: String },
}

pub async fn parse_config_subcommands(subcmd: AvsCommands, config: &IvyConfig, chain: Chain) -> Result<(), Error> {
    // TODO! We need to decrypt wallet here FIRST
    match subcmd {
        AvsCommands::Optin { avs } => avs::avs_default::opt_in(&avs, chain, config, None).await?,
        // AvsCommands::opt_out { avs } => avs::avs_default::boot_avs(&avs, chain, config, None).await?,
        _ => todo!("Unimplemented"),
    };
    Ok(())
}
