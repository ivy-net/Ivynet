use dialoguer::Password;
use ivynet_core::{avs::commands::AvsCommands, config::IvyConfig, server::handle_avs_command, wallet::IvyWallet};
use tracing::info;

use crate::error::Error;

pub async fn parse_avs_subcommands(subcmd: AvsCommands, config: &IvyConfig) -> Result<(), Error> {
    // Not every AVS instance requires access to a wallet. How best to handle this? Enum variant?
    let password: String = Password::new().with_prompt("Input the password for your stored keyfile").interact()?;
    let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), password)?;
    info!("Avs Command: {subcmd}");
    handle_avs_command(subcmd, config, Some(wallet)).await?;
    Ok(())
}
