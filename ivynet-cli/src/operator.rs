use clap::Parser;
use dialoguer::Password;
use ivynet_core::{
    config::IvyConfig,
    eigen::delegation_manager::DelegationManager,
    ethers::{core::types::Address, types::Chain},
    rpc_management::connect_provider,
    wallet::IvyWallet,
};
use tracing::debug;

use crate::{
    error::Error,
    utils::{parse_chain, unwrap_or_local},
};

#[derive(Parser, Debug, Clone)]
pub enum OperatorCommands {
    #[command(name = "get-details", about = "Get operator details <CHAIN> <<ADDRESS>>")]
    Details { chain: String, opt_address: Option<Address> },
    #[command(name = "get-stake", about = "Get an operator's total delineated stake per strategy <CHAIN> <<ADDRESS>>")]
    Stake { chain: String, opt_address: Option<Address> },
    #[command(name = "get-status", about = "Determine whether an address is a registered operator <CHAIN> <<ADDRESS>>")]
    Status { chain: String, opt_address: Option<Address> },
    #[command(name = "register", about = "Register an operator <CHAIN>")]
    Register { chain: String, delegation_approver: Option<Address>, staker_opt_out_window_blocks: Option<u32> },
}

impl OperatorCommands {
    pub fn chain(&self) -> Chain {
        match self {
            OperatorCommands::Details { chain, .. } => parse_chain(chain),
            OperatorCommands::Stake { chain, .. } => parse_chain(chain),
            OperatorCommands::Status { chain, .. } => parse_chain(chain),
            OperatorCommands::Register { chain, .. } => parse_chain(chain),
        }
    }
}

pub async fn parse_operator_subcommands(subcmd: OperatorCommands, config: &IvyConfig) -> Result<(), Error> {
    let chain = subcmd.chain();
    match subcmd {
        OperatorCommands::Details { opt_address, .. } => {
            let provider = connect_provider(&config.get_rpc_url(chain)?, None).await?;
            let manager = DelegationManager::new(&provider);
            manager.get_operator_details(unwrap_or_local(opt_address, config.clone())?).await?;
        }
        OperatorCommands::Stake { opt_address, .. } => {
            let provider = connect_provider(&config.get_rpc_url(chain)?, None).await?;
            let manager = DelegationManager::new(&provider);
            manager.get_staker_delegatable_shares(unwrap_or_local(opt_address, config.clone())?).await?;
            // TODO: Ok, and what should we do with this map?
        }
        OperatorCommands::Status { opt_address, .. } => {
            let provider = connect_provider(&config.get_rpc_url(chain)?, None).await?;
            let manager = DelegationManager::new(&provider);
            manager.get_operator_status(unwrap_or_local(opt_address, config.clone())?).await?;
        }
        OperatorCommands::Register { delegation_approver, staker_opt_out_window_blocks, .. } => {
            let password: String =
                Password::new().with_prompt("Input the password for your stored ECDSA keyfile").interact()?;
            let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), password)?;
            let earnings_receiver = wallet.address();
            let provider = connect_provider(&config.get_rpc_url(chain)?, Some(wallet)).await?;
            let manager = DelegationManager::new(&provider);

            let delegation_approver = delegation_approver.unwrap_or_else(Address::zero);
            let staker_opt_out_window_blocks = staker_opt_out_window_blocks.unwrap_or(0_u32);
            let metadata_uri = &config.metadata.metadata_uri;
            if metadata_uri.is_empty() {
                // TODO: There's probably a better way to check for valid
                // metadata
                return Err(Error::MetadataUriNotFoundError);
            }
            debug!("Operator register: {delegation_approver:?} | {staker_opt_out_window_blocks} | {metadata_uri}");
            manager.register(earnings_receiver, delegation_approver, staker_opt_out_window_blocks, metadata_uri).await?
        }
    }
    Ok(())
}
