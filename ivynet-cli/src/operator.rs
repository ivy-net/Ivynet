use clap::Parser;
use ethers::{core::types::Address, types::Chain};
use ivynet_core::{config::IvyConfig, eigen::delegation_manager::DelegationManager, rpc_management::connect_provider};
use tracing::debug;

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum OperatorCommands {
    #[command(name = "get-details", about = "Get operator details")]
    Details { address: Address },
    #[command(name = "get-stake", about = "Get an operator's total delineated stake per strategy")]
    Stake { address: Address },
    #[command(name = "get-status", about = "Determine whether an address is a registered operator")]
    Status { address: Address },
    #[command(name = "register", about = "Register an operator")]
    Register { delegation_approver: Option<Address>, staker_opt_out_window_blocks: Option<u32> },
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
        OperatorCommands::Register { delegation_approver, staker_opt_out_window_blocks } => {
            let delegation_approver = delegation_approver.unwrap_or_else(Address::zero);
            let staker_opt_out_window_blocks = staker_opt_out_window_blocks.unwrap_or(0_u32);
            let metadata_uri = &config.metadata.metadata_uri;
            if metadata_uri.is_empty() {
                // TODO: There's probably a better way to check for valid
                // metadata
                return Err(Error::MetadataUriNotFoundError);
            }
            debug!("Operator register: {delegation_approver:?} | {staker_opt_out_window_blocks} | {metadata_uri}");
            manager.register(delegation_approver, staker_opt_out_window_blocks, metadata_uri).await?
        }
    }
    Ok(())
}
