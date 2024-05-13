use clap::Parser;
use ethers_core::utils::format_units;

use ivynet_core::eigen::{
    dgm_info::STRATEGY_LIST,
    {self},
};

#[derive(Parser, Debug, Clone)]
pub(crate) enum OperatorCommands {
    #[command(name = "get-details", about = "Get operator details")]
    GetOperatorDetails { address: String },
    #[command(name = "get-stake", about = "Get an operator's total delineated stake per strategy")]
    GetOperatorStake { address: String },
    #[command(name = "get-status", about = "Determine whether an address is a registered operator")]
    GetOperatorStatus { address: String },
}

pub async fn parse_operator_subcommands(subcmd: OperatorCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        OperatorCommands::GetOperatorDetails { address } => {
            eigen::delegation_manager::get_operator_details(&address).await?;
        }
        OperatorCommands::GetOperatorStake { address } => {
            let stake_map = eigen::delegation_manager::get_all_statregies_delegated_stake(address).await?;
            for i in 0..STRATEGY_LIST.len() {
                let stake = stake_map.get(&STRATEGY_LIST[i]).unwrap();
                println!("Share Type: {:?}, Amount: {:?}", STRATEGY_LIST[i], format_units(*stake, "ether").unwrap());
            }
        }
        OperatorCommands::GetOperatorStatus { address } => {
            eigen::delegation_manager::get_operator_status(&address).await?;
        }
    }
    Ok(())
}
