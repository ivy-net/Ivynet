use dialoguer::{Input, MultiSelect, Password, Select};
use ivynet_core::{
    config::IvyConfig,
    dialog::get_confirm_password,
    error::IvyError,
    grpc::{
        backend::backend_client::BackendClient,
        client::{create_channel, Source, Uri},
        messages::RegistrationCredentials,
        tonic::Request,
    },
    keychain::{KeyType, Keychain},
    metadata::Metadata,
    wallet::IvyWallet,
};
use std::{fs, path::PathBuf, unreachable};

#[allow(unused_imports)]
use tracing::debug;

// TODO: Step through piecemeal running/initialization of an empty ivy-config file to ensure
// sensible error messages throughout

pub async fn initialize_ivynet(
    server_url: Uri,
    server_ca: Option<String>,
    skip_login: bool,
) -> Result<(), IvyError> {
    // Build IvyConfig file
    println!("Performing ivynet intialization...");

    let config = IvyConfig::new();
    if config.get_file().exists() {
        let overwrite = Select::new()
            .with_prompt("An ivynet config file already exists. Would you like to overwrite it, overwrite it and create a backup, or exit?")
            .default(0)
            .items(&["Overwrite", "Overwrite and backup", "Exit"])
            .interact()
            .unwrap();
        if overwrite == 0 {
        } else if overwrite == 1 {
            let backup_path = config.get_path().join("ivy-config.toml.bak");
            println!("Backing up existing ivynet config file to {}", backup_path.display());
            fs::copy(config.get_file(), backup_path)?;
        } else {
            return Ok(());
        }
    }

    let setup_types = ["Interactive", "Empty"];
    let interactive = Select::new()
        .with_prompt(
            "Would you like to perform setup in interactive mode, or generate an empty config?",
        )
        .default(0)
        .items(&setup_types)
        .interact()
        .unwrap();
    if interactive == 1 {
        // Empty config
        create_config_dir(config.get_path())?;
        config.store()?;
        println!("An empty ivynet project has been created at {}", config.get_path().display())
    } else if interactive == 0 {
        create_config_dir(config.get_path())?;
        config.store()?;

        // configure RPC addresses
        let config = set_config_rpcs(config)?;
        let config = set_config_keys(config)?;
        // let config = set_config_metadata(config)?;
        config.store()?;

        if !skip_login {
            let config = set_backend_connection(config, server_url, server_ca).await?;
            config.store()?;
        }
    }

    println!("\n----- IvyNet initialization complete -----");
    println!("You can now run `ivynet serve` to start the IvyNet service.");
    println!("You can also run `ivynet config` to view your configuration, or look in the ~/.ivynet directory.");
    println!("------------------------------------------\n");
    Ok(())
}

#[allow(dead_code)]
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

async fn set_backend_connection(
    mut config: IvyConfig,
    server_url: Uri,
    mut server_ca: Option<String>,
) -> Result<IvyConfig, IvyError> {
    let client_key = match config.identity_wallet() {
        Ok(key) => key.address(),
        _ => {
            let new_key = IvyWallet::new();
            config.backend_info.identity_key = new_key.to_private_key();
            new_key.address()
        }
    };

    println!("Server URL: {}", server_url);
    if server_url.to_string().is_empty() {
        // Ask for server URL
        let server_url: String = Input::new()
            .with_prompt("Enter the URL of the IvyNet server you wish to connect to")
            .interact()?;
        config.backend_info.server_url = server_url;
    }

    if server_ca.is_none() {
        // Ask for server CA
        let input_ca: String = Input::new()
            .with_prompt("Enter the path to the server's CA certificate (leave blank to bypass)")
            .allow_empty(true)
            .interact_text()?;
        server_ca = if input_ca.is_empty() { None } else { Some(input_ca) };
        config.backend_info.server_ca = server_ca.clone().unwrap_or("".to_string());
    }

    let email = Input::new()
        .with_prompt("Provide email address to IvyNet system")
        .interact_text()
        .expect("No no email provided");
    let password = Password::new()
        .with_prompt("Enter a password to IvyNet system")
        .interact()
        .expect("No password provided");
    let mut backend =
        BackendClient::new(create_channel(Source::Uri(server_url), server_ca.clone()).await?);
    let hostname = { String::from_utf8(rustix::system::uname().nodename().to_bytes().to_vec()) }
        .expect("Cannot fetch hostname from the node");
    backend
        .register(Request::new(RegistrationCredentials {
            email,
            password,
            hostname,
            public_key: client_key.as_bytes().to_vec(),
        }))
        .await?;

    println!("Node properly registered with key {:?}", client_key);
    Ok(config)
}

fn set_config_keys(mut config: IvyConfig) -> Result<IvyConfig, IvyError> {
    let key_config_types = ["Import", "Create", "Skip"];
    let interactive = Select::new()
        .with_prompt(
            "Would you like to import a private key, create a new private key, or skip this step?",
        )
        .items(&key_config_types)
        .default(0)
        .interact()
        .unwrap();
    match interactive {
        0 => {
            let private_key: String =
                Password::new().with_prompt("Enter your ECDSA private key").interact()?;
            let keyfile_name: String =
                Input::new().with_prompt("Enter a name for the keyfile").interact()?;
            let pw = get_confirm_password();
            let keychain = Keychain::default();
            let key = keychain.import(KeyType::Ecdsa, Some(&keyfile_name), &private_key, &pw)?;

            if let Some(wallet) = key.get_wallet_owned() {
                config.default_ecdsa_keyfile = Some(keyfile_name);
                config.default_ecdsa_address = wallet.address();
            }
        }
        1 => {
            let keyfile_name: String =
                Input::new().with_prompt("Enter a name for the keyfile").interact()?;
            let mut pw: String = Password::new()
                .with_prompt("Enter a password for keyfile encryption")
                .interact()?;
            let mut confirm_pw: String =
                Password::new().with_prompt("Confirm keyfile password").interact()?;

            let mut pw_confirmed = pw == confirm_pw;
            while !pw_confirmed {
                println!("Password and confirmation do not match. Please retry.");
                pw = Password::new()
                    .with_prompt("Enter a password for keyfile encryption")
                    .interact()?;
                confirm_pw = Password::new().with_prompt("Confirm keyfile password").interact()?;
                pw_confirmed = pw == confirm_pw;
            }
            let keychain = Keychain::default();
            let key = keychain.generate(KeyType::Ecdsa, Some(&keyfile_name), &pw);
            if let Some(wallet) = key.get_wallet_owned() {
                config.default_ecdsa_keyfile = Some(keyfile_name);
                let addr = wallet.address();
                config.default_ecdsa_address = addr;
                println!("Public Address: {:?}", addr)
            }
        }
        2 => {
            println!("Skipping keyfile initialization");
        }
        _ => unreachable!("Unknown key setup option reached"),
    }
    Ok(config)
}

fn create_config_dir(config_path: PathBuf) -> Result<(), IvyError> {
    if !config_path.exists() {
        fs::create_dir_all(&config_path)?;
    }
    Ok(())
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::{future::Future, path::PathBuf};
    use tokio::fs;

    pub async fn build_test_dir<F, Fut, T>(test_dir: &str, test_logic: F) -> T
    where
        F: FnOnce(PathBuf) -> Fut,
        Fut: Future<Output = T>,
    {
        let test_path = std::env::current_dir().unwrap().join(format!("testing{}", test_dir));
        fs::create_dir_all(&test_path).await.expect("Failed to create testing_temp directory");
        let result = test_logic(test_path.clone()).await;
        fs::remove_dir_all(test_path).await.expect("Failed to delete testing_temp directory");

        result
    }

    #[tokio::test]
    async fn test_config_file_builds_init() {
        build_test_dir("test_initialization", |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());
            config.store().expect("Config not working");
            let config_file_path = test_path.join("ivy-config.toml");
            assert!(config_file_path.exists());
        })
        .await;
    }
}
