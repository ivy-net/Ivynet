use dialoguer::Password;
use ivynet_core::{
    avs::{build_avs_provider, commands::AvsCommands},
    config::IvyConfig,
    grpc::client::{create_channel, Source},
    wallet::IvyWallet,
};

use crate::{client::IvynetClient, error::Error};

pub async fn parse_avs_subcommands(subcmd: AvsCommands, config: &IvyConfig) -> Result<(), Error> {
    let sock = Source::Path(config.uds_dir());
    let mut client = IvynetClient::from_channel(create_channel(sock, None).await?);
    match subcmd {
        AvsCommands::Info {} => {
            let response = client.avs_mut().avs_info().await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Setup { ref avs, ref chain } => {
            let password: String = Password::new()
                .with_prompt("Input the password for your stored keyfile")
                .interact()?;
            let wallet =
                IvyWallet::from_keystore(config.default_private_keyfile.clone(), &password)?;
            let avs =
                build_avs_provider(Some(avs), chain, config, Some(wallet), Some(password)).await?;
            avs.setup(config).await?;
        }
        // TODO: Fix timeout issue
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

// TODO:
// Check error on following flows:
// - Start without setup (currently returns "no such file")
