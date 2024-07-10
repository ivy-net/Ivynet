use clap::Parser;
use ivynet_core::{
    config::IvyConfig,
    ethers::core::types::Address,
    grpc::client::{create_channel, Source},
};
use std::path::PathBuf;
use tracing::debug;

use crate::{client::IvynetClient, error::Error};

#[derive(Parser, Debug, Clone)]
pub enum OperatorCommands {
    #[command(name = "get", about = "Get operator information")]
    Get {
        #[command(subcommand)]
        subcmd: OperatorGetterCommands,
    },
    #[command(name = "register", about = "Register an operator <CHAIN>")]
    Register {
        chain: String,
        delegation_approver: Option<Address>,
        staker_opt_out_window_blocks: Option<u32>,
    },
    #[command(name = "set", about = "Set operator information")]
    Set {
        #[command(subcommand)]
        subcmd: OperatorSetterCommands,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum OperatorGetterCommands {
    #[command(name = "details", about = "Get operator details for loaded operator <<ADDRESS>>")]
    Details { opt_address: Option<Address> },
    #[command(name = "shares", about = "Get an operator's total shares per strategy <<ADDRESS>>")]
    Shares { opt_address: Option<Address> },
    #[command(
        name = "delegatable-shares",
        about = "Get an operator's shares per strategy available for delegation <<ADDRESS>>"
    )]
    DelegatableShares { opt_address: Option<Address> },
}

#[derive(Parser, Debug, Clone)]
pub enum OperatorSetterCommands {
    #[command(
        name = "ecdsa-keyfile",
        about = "Set ECDSA keyfile path for your operator <KEYFILE_PATH>"
    )]
    EcdsaKeyfile { ecdsa_keypath: PathBuf },
    #[command(
        name = "ecdsa-keyfile",
        about = "Set ECDSA keyfile path for your operator <KEYFILE_PATH>"
    )]
    BlsKeyfile { bls_keypath: PathBuf },
}

pub async fn parse_operator_subcommands(
    subcmd: OperatorCommands,
    config: &IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        OperatorCommands::Get { subcmd } => {
            parse_operator_getter_subcommands(subcmd, config).await?;
        }
        OperatorCommands::Set { subcmd } => {
            parse_operator_setter_subcommands(subcmd, config).await?;
        }
        OperatorCommands::Register { .. } => {
            todo!();
            // let password: String = Password::new()
            //     .with_prompt("Input the password for your stored ECDSA keyfile")
            //     .interact()?;
            // let wallet =
            //     IvyWallet::from_keystore(config.default_private_keyfile.clone(), &password)?;
            // let earnings_receiver = wallet.address();
            // let provider = connect_provider(&config.get_rpc_url(chain)?, Some(wallet)).await?;
            // let manager = DelegationManager::new(Arc::new(provider))?;

            // let delegation_approver = delegation_approver.unwrap_or_else(Address::zero);
            // let staker_opt_out_window_blocks = staker_opt_out_window_blocks.unwrap_or(0_u32);
            // let metadata_uri = &config.metadata.metadata_uri;
            // if metadata_uri.is_empty() {
            //     // TODO: There's probably a better way to check for valid
            //     // metadata
            //     return Err(Error::MetadataUriNotFoundError);
            // }
            // debug!("Operator register: {delegation_approver:?} | {staker_opt_out_window_blocks} | {metadata_uri}");
            // manager
            //     .register(
            //         earnings_receiver,
            //         delegation_approver,
            //         staker_opt_out_window_blocks,
            //         metadata_uri,
            //     )
            //     .await?;
        }
    }
    Ok(())
}

pub async fn parse_operator_getter_subcommands(
    subgetter: OperatorGetterCommands,
    config: &IvyConfig,
) -> Result<(), Error> {
    let sock = Source::Path(config.uds_dir());
    let mut client = IvynetClient::from_channel(create_channel(sock, None).await?);
    match subgetter {
        OperatorGetterCommands::Details { .. } => {
            let response = client.operator_mut().get_operator_details().await?;
            println!("{:?}", response.into_inner());
        }
        OperatorGetterCommands::Shares { .. } => {
            let response = client.operator_mut().get_operator_shares().await?;
            println!("{:?}", response.into_inner());
        }
        OperatorGetterCommands::DelegatableShares { .. } => {
            let response = client.operator_mut().get_delegatable_shares(None).await?;
            println!("{:?}", response.into_inner());
        }
    }
    Ok(())
}

pub async fn parse_operator_setter_subcommands(
    subsetter: OperatorSetterCommands,
    config: &IvyConfig,
) -> Result<(), Error> {
    let sock = Source::Path(config.uds_dir());
    let mut client = IvynetClient::from_channel(create_channel(sock, None).await?);
    match subsetter {
        OperatorSetterCommands::EcdsaKeyfile { ecdsa_keypath } => {
            //client.operator_mut().set_ecdsa_keyfile_path(ecdsa_keypath).await?
            todo!();
        }
        OperatorSetterCommands::BlsKeyfile { bls_keypath } => {
            //client.operator_mut().set_bls_keyfile_path(bls_keypath).await?
            todo!();
        }
    }
    Ok(())
}
