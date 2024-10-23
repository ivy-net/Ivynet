use anyhow::{Context, Error as AnyError, Result};
use dialoguer::Password;
use ivynet_core::{
    avs::{build_avs_provider, commands::AvsCommands, config::AvsConfig},
    config::IvyConfig,
    error::IvyError,
    grpc::client::{create_channel, Source},
    keychain::{KeyType, Keychain},
};

use crate::{client::IvynetClient, error::Error, inspect::tail_logs};

pub async fn parse_avs_subcommands(
    subcmd: AvsCommands,
    config: &IvyConfig,
) -> Result<(), AnyError> {
    let sock = Source::Path(config.uds_dir());

    // Setup runs local, otherwise construct a client and continue.
    if let AvsCommands::Setup { ref avs, ref chain } = subcmd {
        let keychain = Keychain::default();
        let ecdsa_keyname = keychain.select_key(KeyType::Ecdsa).map_err(|e| match e {
            IvyError::NoKeyFoundError => Error::NoECDSAKey,
            e => e.into(),
        })?;

        let ecdsa_password: String = Password::new()
            .with_prompt("Input the password for your stored operator ECDSA keyfile")
            .interact()?;

        let ecdsa = keychain.load(ecdsa_keyname, &ecdsa_password)?;
        let bls_keyname = keychain.select_key(KeyType::Bls).map_err(|e| match e {
            IvyError::NoKeyFoundError => Error::NoBLSKey,
            e => e.into(),
        })?;

        let bls_password: String = Password::new()
            .with_prompt("Input the password for your stored operator Bls keyfile")
            .interact()?;

        if let Some(wallet) = ecdsa.get_wallet_owned() {
            let mut avs = build_avs_provider(
                Some(avs),
                chain,
                config,
                Some(wallet),
                Some(ecdsa_password.clone()),
                None,
            )
            .await?;
            avs.setup(config, Some(ecdsa_password), &format!("{bls_keyname}"), &bls_password)
                .await
                .map_err(|e| match e {
                    IvyError::NoKeyFoundError => Error::NoBLSKey,
                    e => e.into(),
                })?;
        } else {
            println!("Error loading keys");
        }
        return Ok(());
    }

    if let AvsCommands::Inspect { avs, chain, log } = subcmd {
        let (avs, chain) = if let (Some(avs), Some(chain)) = (avs, chain) {
            (avs, chain)
        } else {
            let mut client = IvynetClient::from_channel(create_channel(sock, None).await?);
            let info = client.avs_mut().avs_info().await?.into_inner();
            let avs = info.avs_type;
            let chain = info.chain;
            if avs == "None" || chain == "None" {
                return Err(Error::NoAvsSelectedLogError.into());
            }
            (avs.to_owned(), chain.to_owned())
        };

        // let mut avs = build_avs_provider(Some(&avs), &chain, config, None, None).await?;
        let log_dir = AvsConfig::log_path(&avs, &chain);
        let log_filename = format!("{}.log", log);
        let log_file = log_dir.join(log_filename);
        tail_logs(log_file, 100).await?;

        return Ok(());
    }

    let channel = create_channel(sock, None).await.context("Failed to connect to the ivynet daemon. Please ensure the daemon is running and is connected to ~/.ivynet/ivynet.ipc")?;
    let mut client = IvynetClient::from_channel(channel);
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
        _ => unimplemented!("Command not implemented: {:?}", subcmd),
    }
    Ok(())
}

// TODO:
// Check error on following flows:
// - Start without setup (currently returns "no such file")
