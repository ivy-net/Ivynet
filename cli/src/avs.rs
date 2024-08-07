use dialoguer::Password;
use ivynet_core::{
    avs::{build_avs_provider, commands::AvsCommands},
    config::IvyConfig,
    error::IvyError,
    grpc::client::{create_channel, Source},
    keyring::Keyring,
    wallet::IvyWallet,
};

use crate::{client::IvynetClient, error::Error};

pub async fn parse_avs_subcommands(subcmd: AvsCommands, config: &IvyConfig) -> Result<(), Error> {
    let sock = Source::Path(config.uds_dir());

    // Setup runs local, otherwise construct a client and continue.
    if let AvsCommands::Setup { ref avs, ref chain } = subcmd {
        // TODO: Attempt env
        let keyring = Keyring::load_default().map_err(|e| IvyError::from(e))?;
        let keyfile = keyring.default_ecdsa_keyfile().map_err(|e| IvyError::from(e))?;
        let (wallet, password) =
            keyfile.try_to_wallet_env_dialog().map_err(|e| IvyError::from(e))?;

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

// TODO:
// Check error on following flows:
// - Start without setup (currently returns "no such file")
