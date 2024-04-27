use clap::Parser;

use crate::{avs_info, rpc_management::Network};

#[derive(Parser, Debug, Clone)]
pub enum AvsCommands {
    #[command(name = "boot", about = "Boot up an AVS node with the given AVS")]
    Boot { avs: String },
    #[command(
        name = "check-stake-percentage",
        about = "Determine what percentage of the total stake an address would have"
    )]
    CheckStakePercentage {
        avs: String,
        address: String,
        network: String,
    },
}

pub async fn parse_config_subcommands(subcmd: AvsCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        AvsCommands::Boot { avs } => {
            println!("Booting up AVS: {}", avs);
            avs_info::avs_default::boot_avs(&avs).await?
        }
        AvsCommands::CheckStakePercentage { avs, address, network } => {
            println!("Checking stake percentage for address: {}", address);
            let net = Network::from(network.as_str());
            avs_info::avs_default::check_stake_and_system_requirements(&avs, &address, net).await?;
        }
    };
    Ok(())
}
