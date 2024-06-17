use dialoguer::{Input, MultiSelect, Password, Select};
use ivynet_core::wallet::IvyWallet;
use ivynet_core::{config::IvyConfig, error::IvyError};
use std::unreachable;
use std::{fs, path::PathBuf};
use tracing::debug;

// TODO: Step through piecemeal running/initialization of an empty ivy-config file to ensure
// sensible error messages throughout

pub fn initialize_ivynet() -> Result<(), IvyError> {
    // Build IvyConfig file
    println!("Performing ivynet intialization...");
    let setup_types = ["Interactive", "Empty"];
    let interactive = Select::new()
        .with_prompt("Would you like to perform setup in interactive mode, or generate an empty config?")
        .items(&setup_types)
        .interact()
        .unwrap();
    if interactive == 1 {
        // Empty config
        let config = IvyConfig::new();
        create_config_dir(config.get_path())?;
        config.store()?;
        println!("An empty ivynet project has been created at {}", config.get_path().display())
    } else if interactive == 0 {
        let config = IvyConfig::new();
        create_config_dir(config.get_path())?;
        config.store()?;

        // configure RPC addresses
        let config = set_config_rpcs(config)?;
        let config = set_config_keys(config)?;
        config.store()?;
    }
    Ok(())
}

fn set_config_rpcs(mut config: IvyConfig) -> Result<IvyConfig, IvyError> {
    let mainnet_text = format!("mainnet (default: {})", config.mainnet_rpc_url);
    let testnet_text = format!("holesky (default: {})", config.holesky_rpc_url);
    let local_text = format!("local (default: {})", config.local_rpc_url);
    let rpc_options = [mainnet_text, testnet_text, local_text];
    let rpcs_to_set = MultiSelect::new()
        .with_prompt("Select the network RPCs you wish to configure. Press space to toggle flag, Enter to confirm.")
        .items(&rpc_options)
        .interact()
        .unwrap();

    if rpcs_to_set.is_empty() {
        println!("No RPCs selected, using default values.");
    }

    for res in rpcs_to_set {
        match res {
            0 => {
                let new_rpc =
                    Input::<String>::new().with_prompt("Enter your Mainnet RPC URL:").interact_text().unwrap();
                config.mainnet_rpc_url = new_rpc;
            }
            1 => {
                let new_rpc =
                    Input::<String>::new().with_prompt("Enter your Holesky RPC URL:").interact_text().unwrap();
                config.holesky_rpc_url = new_rpc;
            }
            2 => {
                let new_rpc = Input::<String>::new().with_prompt("Enter your Local RPC URL:").interact_text().unwrap();
                config.local_rpc_url = new_rpc;
            }
            _ => unreachable!("Unknown RPC key reached"),
        }
    }

    Ok(config)
}

fn set_config_keys(mut config: IvyConfig) -> Result<IvyConfig, IvyError> {
    let key_config_types = ["Import", "Create", "Skip"];
    let interactive = Select::new()
        .with_prompt("Would you like to import a private key, create a new private key, or skip this step?")
        .items(&key_config_types)
        .interact()
        .unwrap();
    match interactive {
        0 => {
            let private_key: String = Password::new().with_prompt("Enter your ECDSA private key").interact()?;
            let keyfile_name: String = Input::new().with_prompt("Enter a name for the keyfile").interact()?;
            let mut pw: String = Password::new().with_prompt("Enter a password for keyfile encryption").interact()?;
            let mut confirm_pw: String = Password::new().with_prompt("Confirm keyfile password").interact()?;

            let mut pw_confirmed = pw == confirm_pw;
            while !pw_confirmed {
                println!("Password and confirmation do not match. Please retry.");
                pw = Password::new().with_prompt("Enter a password for keyfile encryption").interact()?;
                confirm_pw = Password::new().with_prompt("Confirm keyfile password").interact()?;
                pw_confirmed = pw == confirm_pw;
            }
            let wallet = IvyWallet::from_private_key(private_key)?;
            let (pub_key_path, prv_key_path) = wallet.encrypt_and_store(keyfile_name, pw)?;
            config.default_public_keyfile = pub_key_path;
            config.default_private_keyfile.clone_from(&prv_key_path);
        }
        1 => {
            let wallet = IvyWallet::new();
            let addr = wallet.address();
            println!("Public Address: {:?}", addr);

            let keyfile_name: String = Input::new().with_prompt("Enter a name for the keyfile").interact()?;
            let mut pw: String = Password::new().with_prompt("Enter a password for keyfile encryption").interact()?;
            let mut confirm_pw: String = Password::new().with_prompt("Confirm keyfile password").interact()?;

            let mut pw_confirmed = pw == confirm_pw;
            while !pw_confirmed {
                println!("Password and confirmation do not match. Please retry.");
                pw = Password::new().with_prompt("Enter a password for keyfile encryption").interact()?;
                confirm_pw = Password::new().with_prompt("Confirm keyfile password").interact()?;
                pw_confirmed = pw == confirm_pw;
            }

            let (pub_key_path, prv_key_path) = wallet.encrypt_and_store(keyfile_name, pw)?;
            config.default_public_keyfile = pub_key_path;
            config.default_private_keyfile.clone_from(&prv_key_path);
        }
        2 => {
            println!("Skipping keyfile initialization");
        }
        _ => unreachable!("Unknown key setup option reached"),
    }
    Ok(config)
}

fn create_config_dir(mut config_path: PathBuf) -> Result<(), IvyError> {
    config_path.pop();
    if !config_path.exists() {
        fs::create_dir_all(&config_path)?;
    }
    Ok(())
}

#[cfg(test)]
pub mod test {
    use super::*;
    #[test]
    fn test_create_config_dir() {
        let mut config_path = PathBuf::from("test_path/test_config.toml");
        create_config_dir(config_path.clone()).unwrap();
        config_path.pop();
        assert!(config_path.exists());
        fs::remove_dir_all(config_path).unwrap();
    }
}
