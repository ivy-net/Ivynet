use anyhow::{Context, Error as AnyError, Result};
use dialoguer::{Password, Select};
use ivynet_core::{
    avs::{build_avs_provider, commands::AvsCommands, config::AvsConfig},
    config::IvyConfig,
    error::IvyError,
    ethers::types::H160,
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

        let wallet_address = keychain.public_address(ecdsa_keyname.clone())?.parse::<H160>()?;

        let setup_options = ["New Deployment", "Custom Attachment"];
        let setup_type = Select::new()
            .with_prompt(format!("Do you have an existing deployment of {}?", avs))
            .items(&setup_options)
            .default(0)
            .interact()
            .unwrap();

        let bls_data = match setup_type {
            0 => {
                let bls_keyname = keychain.select_key(KeyType::Bls).map_err(|e| match e {
                    IvyError::NoKeyFoundError => Error::NoBLSKey,
                    e => e.into(),
                })?;

                let bls_password: String = Password::new()
                    .with_prompt("Input the password for your stored operator Bls keyfile")
                    .interact()?;

                Some((format!("{bls_keyname}"), bls_password))
            }
            _ => None,
        };

        let mut avs = build_avs_provider(Some(avs), chain, config, None, None, None).await?;
        avs.setup(config, wallet_address, bls_data).await.map_err(|e| match e {
            IvyError::NoKeyFoundError => Error::NoBLSKey,
            e => e.into(),
        })?;

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
            let keychain = Keychain::default();
            let ecdsa_keyname = keychain.select_key(KeyType::Ecdsa).map_err(|e| match e {
                IvyError::NoKeyFoundError => Error::NoECDSAKey,
                e => e.into(),
            })?;

            let ecdsa_password: String = Password::new()
                .with_prompt("Input the password for your stored operator ECDSA keyfile")
                .interact()?;
            // We need to check if we can load the key with this password
            if keychain.load(ecdsa_keyname.clone(), &ecdsa_password).is_ok() {
                let response =
                    client.avs_mut().register(format!("{ecdsa_keyname:?}"), ecdsa_password).await?;

                println!("{:?}", response.into_inner());
            } else {
                println!("ERROR: Bad password to selected key");
            }
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
