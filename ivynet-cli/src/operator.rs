use clap::Parser;
use ivynet_core::{
    config::IvyConfig,
    eigen::delegation_manager::DelegationManager,
    ethers::{core::types::Address, types::Chain},
    rpc_management::connect_provider,
};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum OperatorCommands {
    #[command(name = "get-details", about = "Get operator details")]
    Details { address: Address },
    #[command(name = "get-stake", about = "Get an operator's total delineated stake per strategy")]
    Stake { address: Address },
    #[command(name = "get-status", about = "Determine whether an address is a registered operator")]
    Status { address: Address },
}

pub async fn parse_operator_subcommands(
    subcmd: OperatorCommands,
    config: &IvyConfig,
    chain: Chain,
) -> Result<(), Error> {
    let provider = connect_provider(&config.get_rpc_url(chain)?, None).await?;
    let manager = DelegationManager::new(&provider);
    match subcmd {
        OperatorCommands::Details { address } => {
            manager.get_operator_details(address).await?;
        }
        OperatorCommands::Stake { address } => {
            let _stake_map = manager.get_staker_delegatable_shares(address).await?;
            // TODO: Ok, and what should we do with this map?
        }
        OperatorCommands::Status { address } => {
            manager.get_operator_status(address).await?;
        }
    }
    Ok(())
}
