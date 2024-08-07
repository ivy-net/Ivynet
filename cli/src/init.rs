use dialoguer::{Input, MultiSelect, Password, Select};
use ivynet_core::{
    config::IvyConfig,
    dialog::get_confirm_password,
    error::IvyError,
    keyring::{Keyring, DEFAULT_KEY_ID},
    metadata::Metadata,
    wallet::IvyWallet,
};
use std::{fs, path::PathBuf, unreachable};
use tracing::info;

#[allow(unused_imports)]
use tracing::debug;

// TODO: Step through piecemeal running/initialization of an empty ivy-config file to ensure
// sensible error messages throughout

pub fn initialize_ivynet() -> Result<(), IvyError> {
    // Build IvyConfig file
    println!("Performing ivynet intialization...");
    let setup_types = ["Interactive", "Empty"];
    let interactive = Select::new()
        .with_prompt(
            "Would you like to perform setup in interactive mode, or generate an empty config?",
        )
        .items(&setup_types)
        .interact()
        .unwrap();
    // Initialize keyring
    let keyring = if !Keyring::default_path().exists() {
        Keyring::default()
    } else {
        Keyring::load(Keyring::default_path())?
    };

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
        let config = set_config_metadata(config)?;
        config.store()?;

        let keyring = set_config_keyring(keyring)?;
        keyring.store()?;
    }
    Ok(())
}

fn set_config_metadata(mut config: IvyConfig) -> Result<IvyConfig, IvyError> {
    let mut metadata = Metadata::default();
    let metadata_fields = ["Metadata URI", "Logo URI", "Favicon URI"];
    let fields_to_fill = MultiSelect::new()
        .with_prompt("Select the metadata fields you wish to configure. Press space to toggle flag, Enter to confirm.")
        .items(&metadata_fields)
        .interact()
        .unwrap();
    for field in fields_to_fill {
        match field {
            0 => {
                let metadata_uri: String =
                    Input::new().with_prompt("Enter the operator metadata URI").interact()?;
                metadata.metadata_uri = metadata_uri;
            }
            1 => {
                let logo_uri: String =
                    Input::new().with_prompt("Enter the operator logo URI").interact()?;
                metadata.logo_uri = logo_uri;
            }
            2 => {
                let favicon_uri: String =
                    Input::new().with_prompt("Enter the operator favicon URI").interact()?;
                metadata.favicon_uri = favicon_uri;
            }
            _ => unreachable!("Unknown metadata field reached"),
        }
    }
    config.metadata = metadata;
    Ok(config)
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
                let new_rpc = Input::<String>::new()
                    .with_prompt("Enter your Mainnet RPC URL:")
                    .interact_text()
                    .unwrap();
                config.mainnet_rpc_url = new_rpc;
            }
            1 => {
                let new_rpc = Input::<String>::new()
                    .with_prompt("Enter your Holesky RPC URL:")
                    .interact_text()
                    .unwrap();
                config.holesky_rpc_url = new_rpc;
            }
            2 => {
                let new_rpc = Input::<String>::new()
                    .with_prompt("Enter your Local RPC URL:")
                    .interact_text()
                    .unwrap();
                config.local_rpc_url = new_rpc;
            }
            _ => unreachable!("Unknown RPC key reached"),
        }
    }

    Ok(config)
}

fn set_config_keyring(mut keyring: Keyring) -> Result<Keyring, IvyError> {
    info!("Initializing keyring...");
    keyring = interactive_add_default_keyfile(keyring)?;
    let mut finished = false;
    while !finished {
        let interactive = Select::new()
            .with_prompt("Would you like to add another keyfile?")
            .items(&["Yes", "No"])
            .interact()
            .unwrap();
        match interactive {
            0 => {
                keyring = interactive_add_new_keyfile(keyring)?;
            }
            1 => {
                finished = true;
            }
            _ => unreachable!("Unknown key setup option reached"),
        }
    }
    Ok(keyring)
}

fn interactive_add_default_keyfile(mut keyring: Keyring) -> Result<Keyring, IvyError> {
    let key_config_types = ["Import", "Create", "Skip"];
    let interactive = Select::new()
        .with_prompt(
            "Would you like to import a default private key, create a new default private key, or skip this step? This can be done later with the `ivynet keys` command.",
        )
        .items(&key_config_types)
        .interact()
        .unwrap();
    match interactive {
        0 => {
            let private_key: String =
                Password::new().with_prompt("Enter your ECDSA private key").interact()?;
            let pw = get_confirm_password();
            let wallet = IvyWallet::from_private_key(private_key)?;
            let prv_key_path =
                wallet.encrypt_and_store(&keyring.keyring_dir()?, &DEFAULT_KEY_NAME, &pw)?;
            keyring.add_ecdsa_keyfile(DEFAULT_KEY_NAME, prv_key_path);
        }
        1 => {
            let wallet = IvyWallet::new();
            let addr = wallet.address();
            println!("Public Address: {:?}", addr);
            let pw = get_confirm_password();
            let prv_key_path = wallet.encrypt_and_store(
                &keyring.keyring_dir()?,
                &DEFAULT_KEY_NAME.to_lowercase(),
                &pw,
            )?;
            let keyfile = EcdsaKeyFile {
                name: DEFAULT_KEY_NAME.to_string(),
                
            }
            keyring.add_ecdsa_keyfile(DEFAULT_KEY_NAME, prv_key_path);
        }
        2 => {
            println!("Skipping keyfile initialization");
        }
        _ => unreachable!("Unknown key setup option reached"),
    }
    Ok(keyring)
}

fn interactive_add_new_keyfile(mut keyring: Keyring) -> Result<Keyring, IvyError> {
    let key_config_types = ["Import", "Create", "Finish"];
    let interactive = Select::new()
        .with_prompt("Would you like to import or create another private key?")
        .items(&key_config_types)
        .interact()
        .unwrap();
    match interactive {
        0 => {
            let private_key: String =
                Password::new().with_prompt("Enter your ECDSA private key").interact()?;
            let keyfile_name: String =
                Input::new().with_prompt("Enter a name for the keyfile").interact()?;
            let pw = get_confirm_password();
            let wallet = IvyWallet::from_private_key(private_key)?;
            let prv_key_path = wallet.encrypt_and_store(
                &keyring.keyring_dir()?,
                &keyfile_name.to_lowercase(),
                &pw,
            )?;
            keyring.add_ecdsa_keyfile(&keyfile_name.to_uppercase(), prv_key_path);
        }
        1 => {
            let wallet = IvyWallet::new();
            let addr = wallet.address();
            println!("Public Address: {:?}", addr);
            let keyfile_name: String =
                Input::new().with_prompt("Enter a name for the keyfile").interact()?;
            let pw = get_confirm_password();
            let prv_key_path = wallet.encrypt_and_store(
                &keyring.keyring_dir()?,
                &keyfile_name.to_lowercase(),
                &pw,
            )?;
            keyring.add_ecdsa_keyfile(&keyfile_name.to_uppercase(), prv_key_path);
        }
        2 => {
            println!("Skipping keyfile initialization");
        }
        _ => unreachable!("Unknown key setup option reached"),
    }
    Ok(keyring)
}

fn create_config_dir(config_path: PathBuf) -> Result<(), IvyError> {
    if !config_path.exists() {
        fs::create_dir_all(&config_path)?;
    }
    Ok(())
}

#[cfg(test)]
pub mod test {}
