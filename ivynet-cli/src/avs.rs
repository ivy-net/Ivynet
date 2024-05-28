use clap::Parser;

use ivynet_core::avs;

#[derive(Parser, Debug, Clone)]
pub enum AvsCommands {
    #[command(name = "boot", about = "Boot up an AVS node with the given AVS")]
    Boot { avs: String },
    #[command(
        name = "check-stake-percentage",
        about = "Determine what percentage of the total stake an address would have"
    )]
    CheckStakePercentage { avs: String, address: String, network: String },
}

pub async fn parse_config_subcommands(subcmd: AvsCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        AvsCommands::Boot { avs } => avs::avs_default::boot_avs(&avs).await?,
        _ => todo!("Unimplemented"),
    };
    Ok(())
}
