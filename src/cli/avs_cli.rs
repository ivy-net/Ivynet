use clap::Parser;

use crate::avs_info;

#[derive(Parser, Debug, Clone)]
pub enum AvsCommands {
    #[command(name = "boot", about = "Boot up an AVS node with the given AVS",)]
    Boot { avs: String },
}

pub async fn parse_config_subcommands(subcmd: AvsCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        AvsCommands::Boot { avs } => {
            println!("Booting up AVS: {}", avs);
            avs_info::avs_default::boot_avs(&avs).await?
        }
    };
    Ok(())
}