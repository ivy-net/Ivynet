use dialoguer::{Password, Select};
use ivynet_core::{
    avs::{build_avs_provider, commands::AvsCommands},
    config::IvyConfig,
    grpc::client::{create_channel, Source},
    wallet::IvyWallet,
};
use std::fs;
use std::path::PathBuf;

use crate::{client::IvynetClient, error::Error};

pub async fn parse_avs_subcommands(subcmd: AvsCommands, config: &IvyConfig) -> Result<(), Error> {
    let sock = Source::Path(config.uds_dir());

    // Setup runs local, otherwise construct a client and continue.
    if let AvsCommands::Setup { ref avs, ref chain } = subcmd {
        let selected_key = &get_keyfile(config.get_key_path(), "ecdsa");
        let keypath = config.get_key_path().join(selected_key);
        println!("{:?}", keypath);

        let password: String = Password::new()
            .with_prompt("Input the password for your stored operator ECDSA keyfile")
            .interact()?;
        let wallet = IvyWallet::from_keystore(keypath.clone(), &password)?;
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
            let response = client.avs_mut().opt_in().await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Unregister {} => {
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
        AvsCommands::Select { avs, chain } => {
            let response = client.avs_mut().select_avs(avs, chain).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::CheckStakePercentage { .. } => todo!(),
        _ => unimplemented!("Command not implemented: {:?}", subcmd),
    }
    Ok(())
}

fn get_all_keyfiles(dir: PathBuf, extension: &str) -> Vec<String> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name_str) = path.file_name().and_then(|f| f.to_str()) {
                    if file_name_str.ends_with(extension) {
                        files.push(file_name_str.to_string());
                    }
                }
            }
        }
    }
    println!("{:?}", files);
    files.sort();
    files
}

fn get_keyfile( dir: PathBuf, extension: &str) -> String{

    let keys = get_all_keyfiles(dir, &format!("{}.key.json", extension));
    let interactive = Select::new()
        .with_prompt(
            format!("Which {} key would you like to select?", extension)
        )
        .items(&keys)
        .interact()
        .unwrap();

    keys[interactive].clone()
}

// TODO:
// Check error on following flows:
// - Start without setup (currently returns "no such file")
