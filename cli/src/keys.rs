use crate::error::Error;
use clap::Parser;
use ivynet_core::{error::IvyError, keyring::Keyring};

#[derive(Parser, Debug, Clone)]
pub enum KeyCommands {
    #[command(name = "list", about = "list all keys in the ivy keyring")]
    ListEcdsa,
    #[command(name = "list-bls", about = "list all bls keys in the ivy keyring")]
    ListBls,
    #[command(name = "add-ecdsa", about = "add a ecdsa key to the ivy keyring")]
    AddEcdsa,
    #[command(name = "add-bls", about = "add a bls key to the ivy keyring")]
    AddBls,
}

pub async fn parse_key_subcommands(subcmd: KeyCommands) -> Result<(), Error> {
    match subcmd {
        KeyCommands::ListEcdsa => {
            let keyring = Keyring::load_default().map_err(IvyError::from)?;
            let all_ecdsa_keys: Vec<_> = keyring.ecdsa_keys.values().collect();
            println!("ECDSA Keys: {:?}", all_ecdsa_keys);
            Ok(())
        }
        KeyCommands::ListBls => {
            let keyring = Keyring::load_default().map_err(IvyError::from)?;
            let all_bls_keys: Vec<_> = keyring.bls_keys.values().collect();
            println!("BLS Keys: {:?}", all_bls_keys);
            Ok(())
        }
        KeyCommands::AddEcdsa => {
            let keyring = Keyring::load_default().map_err(IvyError::from)?;
            let keyfile = keyring.default_ecdsa_keyfile().map_err(IvyError::from)?;
            keyring.add_ecdsa_keyfile(&keyfile.name, keyfile.path).map_err(IvyError::from)?;
            keyring.store().map_err(IvyError::from)?;
            Ok(())
        }
        KeyCommands::AddBls => {
            let keyring = Keyring::load_default().map_err(IvyError::from)?;
            let keyfile = keyring.default_bls_keyfile().map_err(IvyError::from)?;
            keyring.add_bls_keyfile(&keyfile.name, keyfile.path).map_err(IvyError::from)?;
            keyring.store().map_err(IvyError::from)?;
            Ok(())
        }
    }
}
