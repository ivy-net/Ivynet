use clap::Parser;
use ivynet_core::{
    config::IvyConfig,
    ethers::core::types::Address,
    grpc::client::{create_channel, Source},
};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum OperatorCommands {
    #[command(name = "get", about = "Get operator information")]
    Get {
        #[command(subcommand)]
        subcmd: OperatorGetterCommands,
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

pub async fn parse_operator_subcommands(
    subcmd: OperatorCommands,
    config: &IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        OperatorCommands::Get { subcmd } => {
            parse_operator_getter_subcommands(subcmd, config).await?;
        }
    }
    Ok(())
}

pub async fn parse_operator_getter_subcommands(
    subgetter: OperatorGetterCommands,
    config: &IvyConfig,
) -> Result<(), Error> {
    // let sock = Source::Path(config.uds_dir());
    // let mut client = IvynetClient::from_channel(create_channel(sock, None).await?);
    // match subgetter {
    //     OperatorGetterCommands::Details { .. } => {
    //         let response = client.operator_mut().get_operator_details().await?;
    //         println!("{:?}", response.into_inner());
    //     }
    //     OperatorGetterCommands::Shares { .. } => {
    //         let response = client.operator_mut().get_operator_shares().await?;
    //         println!("{:?}", response.into_inner());
    //     }
    //     OperatorGetterCommands::DelegatableShares { .. } => {
    //         let response = client.operator_mut().get_delegatable_shares(None).await?;
    //         println!("{:?}", response.into_inner());
    //     }
    // }
    // Ok(())
    todo!()
}
