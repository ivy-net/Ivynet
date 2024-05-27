use clap::Parser;
use ethers_core::types::Address;

use ivynet_core::eigen::delegation_manager::DELEGATION_MANAGER;

#[derive(Parser, Debug, Clone)]
pub(crate) enum OperatorCommands {
    #[command(name = "get-details", about = "Get operator details")]
    Details { address: Address },
    #[command(name = "get-stake", about = "Get an operator's total delineated stake per strategy")]
    Stake { address: Address },
    #[command(name = "get-status", about = "Determine whether an address is a registered operator")]
    Status { address: Address },
}

pub async fn parse_operator_subcommands(subcmd: OperatorCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        OperatorCommands::Details { address } => {
            DELEGATION_MANAGER.get_operator_details(address).await?;
        }
        OperatorCommands::Stake { address } => {
            DELEGATION_MANAGER.get_staker_delegatable_shares(address).await?;
        }
        OperatorCommands::Status { address } => {
            DELEGATION_MANAGER.get_operator_status(address).await?;
        }
    }
    Ok(())
}
