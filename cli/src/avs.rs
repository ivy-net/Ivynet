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

    // Setup runs local, otherwise construct a client and continue.
    if let AvsCommands::Setup { ref avs, ref chain } = subcmd {
        let password: String = Password::new()
            .with_prompt("Input the password for your stored operator ECDSA keyfile")
            .interact()?;
        let wallet = IvyWallet::from_keystore(config.default_ecdsa_keyfile.clone(), &password)?;
        let avs =
            build_avs_provider(Some(avs), chain, config, Some(wallet), Some(password.clone()))
                .await?;
        avs.setup(config, Some(password)).await?;
        return Ok(());
    }

    let mut client = IvynetClient::from_channel(create_channel(sock, None).await?);
    match subcmd {
        AvsCommands::Info {} => {
            let response = client.avs_mut().avs_info().await?;
            println!("{:?}", response.into_inner());
        }
        // TODO: Fix timeout issue
        AvsCommands::Register {} => {
            let response = client.avs_mut().register().await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Unregister {} => {
            let response = client.avs_mut().unregister().await?;
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
        AvsCommands::Select { avs, chain } => {
            let response = client.avs_mut().select_avs(avs, chain).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Attach { avs, chain } => {
            let response = client.avs_mut().attach(avs, chain).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::CheckStakePercentage { .. } => todo!(),
        _ => unimplemented!("Command not implemented: {:?}", subcmd),
    }
    Ok(())
}

// TODO:
// Check error on following flows:
// - Start without setup (currently returns "no such file")
