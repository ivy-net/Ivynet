use dialoguer::Password;
use ivynet_core::{
    avs::{build_avs_provider, commands::AvsCommands, config::AvsConfig},
    config::IvyConfig,
    grpc::client::{create_channel, Source},
    wallet::IvyWallet,
};

use crate::{client::IvynetClient, error::Error, inspect::tail_logs};

pub async fn parse_avs_subcommands(subcmd: AvsCommands, config: &IvyConfig) -> Result<(), Error> {
    let sock = Source::Path(config.uds_dir());

    // Setup runs local, otherwise construct a client and continue.
    if let AvsCommands::Setup { ref avs, ref chain } = subcmd {
        let password: String = Password::new()
            .with_prompt("Input the password for your stored operator ECDSA keyfile")
            .interact()?;
        let wallet = IvyWallet::from_keystore(config.default_ecdsa_keyfile.clone(), &password)?;
        let mut avs =
            build_avs_provider(Some(avs), chain, config, Some(wallet), Some(password.clone()))
                .await?;
        avs.setup(config, Some(password)).await?;
        return Ok(());
    }

    if let AvsCommands::Inspect { avs, chain, log } = subcmd {
        let (avs, chain) = if avs.is_none() || chain.is_none() {
            let mut client = IvynetClient::from_channel(create_channel(sock, None).await?);
            let info = client.avs_mut().avs_info().await?.into_inner();
            let avs = info.avs_type;
            let chain = info.chain;
            if avs == "None" || chain == "None" {
                return Err(Error::NoAvsSelectedLogError);
            }
            (avs.to_owned(), chain.to_owned())
        } else {
            (avs.unwrap(), chain.unwrap())
        };

        // let mut avs = build_avs_provider(Some(&avs), &chain, config, None, None).await?;
        let log_dir = AvsConfig::log_path(&avs, &chain);
        let log_filename = format!("{}.log", log.to_string());
        let log_file = log_dir.join(log_filename);
        tail_logs(log_file, 100).await?;

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
