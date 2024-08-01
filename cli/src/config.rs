use clap::Parser;
use dialoguer::{Input, Password};
use ivynet_core::{
    config::{self, IvyConfig},
    error::IvyError,
    ethers::types::Chain,
    grpc::{
        backend::backend_client::BackendClient,
        client::{create_channel, Request, Source, Uri},
        messages::RegistrationCredentials,
    },
    metadata::Metadata,
    utils::try_parse_chain,
    wallet::IvyWallet,
};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum ConfigCommands {
    #[command(
        name = "import-key",
        about = "Import and save as your default Ethereum private key with a password <PRIVATE_KEY>"
    )]
    ImportPrivateKey { private_key: String, keyname: Option<String>, password: Option<String> },
    #[command(
        name = "create-key",
        about = "Create an Ethereum private key to use with Ivynet and optionally store it with a password"
    )]
    CreatePrivateKey {
        #[arg(long)]
        store: bool,
        keyname: Option<String>,
        password: Option<String>,
    },
    #[command(
        name = "get-default-public",
        about = "Get the current default saved keypair's Ethereum address"
    )]
    GetDefaultEthAddress,
    #[command(name = "get-default-private", about = "Get the current default saved private key")]
    GetDefaultPrivateKey,
    #[command(
        name = "set-rpc",
        about = "Set default URLs to use when connecting to 'mainnet', 'holesky', and 'local' RPC urls <CHAIN> <RPC_URL>"
    )]
    SetRpc { chain: String, rpc_url: String },
    #[command(
        name = "get-rpc",
        about = "Get the current default RPC URL for 'mainnet', 'holesky', or 'local' <CHAIN>"
    )]
    GetRpc { chain: String },
    #[command(
        name = "get-sys-info",
        about = "Get the number of CPU cores, memory, and free disk space on the current machine"
    )]
    #[command(name = "set-metadata", about = "Set metadata for EigenLayer Operator")]
    SetMetadata {
        metadata_uri: Option<String>,
        logo_uri: Option<String>,
        favicon_uri: Option<String>,
    },
    #[command(name = "get-metadata", about = "Get local metadata")]
    GetMetadata,
    #[command(name = "get-config", about = "Get all config data")]
    GetConfig,
    #[command(name = "get-sys-info", about = "Get system information")]
    GetSysInfo,

    #[command(name = "register", about = "Register node on IvyNet server")]
    Register {
        /// Email address registered at IvyNet portal
        #[arg(long, env = "IVYNET_EMAIL")]
        email: String,

        /// Password to IvyNet account
        #[arg(long, env = "IVYNET_PASSWORD")]
        password: String,
    },
}

pub async fn parse_config_subcommands(
    subcmd: ConfigCommands,
    mut config: IvyConfig,
    server_url: Uri,
    server_ca: Option<&String>,
) -> Result<(), Error> {
    match subcmd {
        ConfigCommands::ImportPrivateKey { private_key, keyname, password } => {
            let wallet = IvyWallet::from_private_key(private_key)?;
            let (keyname, pass) = get_credentials(keyname, password);
            let prv_key_path = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;
            config.default_private_keyfile = prv_key_path;
            config.store().map_err(IvyError::from)?;
        }
        ConfigCommands::CreatePrivateKey { store, keyname, password } => {
            let wallet = IvyWallet::new();
            let priv_key = wallet.to_private_key();
            println!("Private key: {:?}", priv_key);
            let addr = wallet.address();
            println!("Public Address: {:?}", addr);
            if store {
                let (keyname, pass) = get_credentials(keyname, password);
                let prv_key_path = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;
                config.default_private_keyfile = prv_key_path;
                config.store().map_err(IvyError::from)?;
            }
        }
        ConfigCommands::SetRpc { chain, rpc_url } => {
            let chain = try_parse_chain(&chain)?;
            config.set_rpc_url(chain, &rpc_url)?;
            config.store().map_err(IvyError::from)?;
        }
        ConfigCommands::GetRpc { chain } => {
            println!(
                "Url for {chain} is {}",
                config.get_rpc_url(chain.parse::<Chain>().expect("Wrong network name provided"))?
            );
        }
        ConfigCommands::GetDefaultEthAddress => {
            println!(
                "Public Key: {:?}",
                IvyWallet::address_from_file(config.default_public_keyfile.clone())?
            );
        }
        ConfigCommands::GetDefaultPrivateKey => {
            let pass =
                Password::new().with_prompt("Enter a password to the private key").interact()?;
            let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), &pass)?;
            println!("Private key: {:?}", wallet.to_private_key());
        }
        ConfigCommands::SetMetadata { metadata_uri, logo_uri, favicon_uri } => {
            let metadata_uri = metadata_uri.unwrap_or("".to_string());
            let logo_uri = logo_uri.unwrap_or("".to_string());
            let favicon_uri = favicon_uri.unwrap_or("".to_string());
            config.metadata = Metadata::new(&metadata_uri, &logo_uri, &favicon_uri);
        }
        ConfigCommands::GetMetadata => {
            let metadata = &config.metadata;
            println!("{metadata:?}");
        }
        ConfigCommands::GetConfig => {
            println!("{config:?}")
        }
        ConfigCommands::GetSysInfo => {
            let (cpus, mem_info, disk_info) = config::get_system_information()?;
            println!(" --- System Information: --- ");
            println!("CPU Cores: {cpus}");
            println!("Memory Information:");
            println!("  Total: {mem_info}");
            println!("Disk Information:");
            println!("  Free: {disk_info}");
            println!(" --------------------------- ");
        }
        ConfigCommands::Register { email, password } => {
            let config = IvyConfig::load_from_default_path().map_err(IvyError::from)?;
            let public_key = config.identity_wallet()?.address();
            let mut backend =
                BackendClient::new(create_channel(Source::Uri(server_url), server_ca).await?);
            backend
                .register(Request::new(RegistrationCredentials {
                    email,
                    password,
                    public_key: public_key.as_bytes().to_vec(),
                }))
                .await?;
            println!("Node registered");
        }
    };
    Ok(())
}

fn get_credentials(keyname: Option<String>, password: Option<String>) -> (String, String) {
    match (keyname, password) {
        (None, None) => (
            Input::new()
                .with_prompt("Enter a name for the key")
                .interact_text()
                .expect("No keyname provided"),
            Password::new()
                .with_prompt("Enter a password to the private key")
                .interact()
                .expect("No password provided"),
        ),
        (None, Some(pass)) => (
            Input::new()
                .with_prompt("Enter a name for the key")
                .interact_text()
                .expect("No keyname provided"),
            pass,
        ),
        (Some(keyname), None) => (
            keyname,
            Password::new()
                .with_prompt("Enter a password to the private key")
                .interact()
                .expect("No password provided"),
        ),
        (Some(keyname), Some(pass)) => (keyname, pass),
    }
}

#[cfg(test)]
mod tests {
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

    // Usage example within an async test
    #[tokio::test]
    async fn test_import_key() {
        let test_dir = "test_import_key";
        build_test_dir(test_dir, |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());

            let result = parse_config_subcommands(
                ConfigCommands::ImportPrivateKey {
                    private_key: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                        .to_string(),
                    keyname: Some("testkey".to_string()),
                    password: Some("password".to_string()),
                },
                config,
                "http://localhost:50051".parse().unwrap(),
                None,
            )
            .await;

            println!("{result:?}",);
            assert!(result.is_ok());
            assert!(test_path.join("testkey.json").exists());
            assert!(test_path.join("testkey.txt").exists());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{config:?}",);

            // Read and parse the TOML file
            let toml_content = fs::read_to_string(test_path.join("ivy-config.toml"))
                .await
                .expect("Failed to read TOML file");
            let toml_data: toml::Value =
                toml::from_str(&toml_content).expect("Failed to parse TOML");

            // Perform assertions on TOML keys and values
            let private_keypath = format!("{}/testkey.json", test_path.to_str().unwrap());
            assert_eq!(
                toml_data["default_private_keyfile"].as_str(),
                Some(private_keypath.as_str())
            );

            let public_keypath = format!("{}/testkey.txt", test_path.to_str().unwrap());
            assert_eq!(toml_data["default_public_keyfile"].as_str(), Some(public_keypath.as_str()));
        })
        .await;
    }

    #[tokio::test]
    async fn test_create_key() {
        let test_dir = "test_create_key";
        build_test_dir(test_dir, |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());

            let result = parse_config_subcommands(
                ConfigCommands::CreatePrivateKey {
                    store: true,
                    keyname: Some("testkey".to_string()),
                    password: Some("password".to_string()),
                },
                config,
                "http://localhost:50051".parse().unwrap(),
                None,
            )
            .await;

            println!("{result:?}",);
            assert!(result.is_ok());
            assert!(test_path.join("testkey.json").exists());
            assert!(test_path.join("testkey.txt").exists());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{config:?}",);

            // Read and parse the TOML file
            let toml_content = fs::read_to_string(test_path.join("ivy-config.toml"))
                .await
                .expect("Failed to read TOML file");
            let toml_data: toml::Value =
                toml::from_str(&toml_content).expect("Failed to parse TOML");

            // Perform assertions on TOML keys and values
            let private_keypath = format!("{}/testkey.json", test_path.to_str().unwrap());
            assert_eq!(
                toml_data["default_private_keyfile"].as_str(),
                Some(private_keypath.as_str())
            );

            let public_keypath = format!("{}/testkey.txt", test_path.to_str().unwrap());
            assert_eq!(toml_data["default_public_keyfile"].as_str(), Some(public_keypath.as_str()));
        })
        .await;
    }

    #[tokio::test]
    async fn test_rpc_functionality() {
        let test_dir = "test_rpc_functionality";
        build_test_dir(test_dir, |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());

            let result = parse_config_subcommands(
                ConfigCommands::SetRpc {
                    chain: "mainnet".to_string(),
                    rpc_url: "http://localhost:8545".to_string(),
                },
                config,
                "http://localhost:50051".parse().unwrap(),
                None,
            )
            .await;

            println!("{result:?}",);
            assert!(result.is_ok());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{config:?}",);

            let result = parse_config_subcommands(
                ConfigCommands::GetRpc { chain: "mainnet".to_string() },
                config,
                "http://localhost:50051".parse().unwrap(),
                None,
            )
            .await;

            println!("{result:?}",);
            assert!(result.is_ok());
        })
        .await;
    }
}
