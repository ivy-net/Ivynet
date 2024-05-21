use clap::Parser;
use ethers_core::{types::Address, utils::format_units};

use ivynet_core::eigen::{delegation_manager::DELEGATION_MANAGER, dgm_info::STRATEGY_LIST};

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
            let stake_map = DELEGATION_MANAGER.get_all_statregies_delegated_stake(address).await?;
            for i in 0..STRATEGY_LIST.len() {
                let stake = stake_map.get(&STRATEGY_LIST[i]).unwrap();
                println!("Share Type: {:?}, Amount: {:?}", STRATEGY_LIST[i], format_units(*stake, "ether").unwrap());
            }
        }
        OperatorCommands::Status { address } => {
            DELEGATION_MANAGER.get_operator_status(address).await?;
        }
    }
    Ok(())
}
