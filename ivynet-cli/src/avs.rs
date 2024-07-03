use std::str::FromStr;

use dialoguer::Password;
use ivynet_core::{
    avs::{build_avs_provider, commands::AvsCommands},
    config::IvyConfig,
    error::IvyError,
    grpc::{client::create_channel, tonic::transport::Uri},
    wallet::IvyWallet,
};
use tracing::info;

use crate::{client::IvynetClient, error::Error};

pub async fn parse_avs_subcommands(subcmd: AvsCommands, config: &IvyConfig) -> Result<(), Error> {
    // Not every AVS instance requires access to a wallet. How best to handle this? Enum variant?
    let mut client = IvynetClient::from_channel(create_channel(
        &Uri::from_str(&config.ivy_daemon_uri).map_err(|_| IvyError::GRPCClientError)?,
        None,
    ));
    match subcmd {
        AvsCommands::Info {} => {
            let response = client.avs_mut().avs_info().await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Setup { ref avs, ref chain } => {
            let password: String =
                Password::new().with_prompt("Input the password for your stored keyfile").interact()?;
            let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), &password)?;
            let avs = build_avs_provider(Some(avs), chain, config, Some(wallet), Some(password)).await?;
            avs.setup(config).await?;
        }
        AvsCommands::Optin {} => {
            let response = client.avs_mut().opt_in().await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Optout {} => {
            let response = client.avs_mut().opt_out().await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Start { avs, chain } => {
            let response = client.avs_mut().start(avs, chain).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Stop {} => {
            let response = client.avs_mut().stop().await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::SetAvs { avs, chain } => {
            let response = client.avs_mut().set_avs(avs, chain).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::CheckStakePercentage { avs, address, network } => todo!(),
    }
    Ok(())
}
