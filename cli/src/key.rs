use clap::Parser;
use dialoguer::{Input, Password};
use ivynet_core::{config::IvyConfig, ethers::types::H160, wallet::IvyWallet};
use serde_json::Value;
use std::{fs, path::PathBuf};
use tracing::{debug, error};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum KeyCommands {
    #[command(name = "import", about = "Import a ECDSA or a BLS key")]
    Import {
        #[command(subcommand)]
        command: ImportCommands,
    },
    #[command(name = "create", about = "Create a ECDSA or a BLS key")]
    Create {
        #[command(subcommand)]
        command: CreateCommands,
    },
    #[command(name = "get", about = "Get ECDSA/BLS private/public key")]
    Get {
        #[command(subcommand)]
        command: GetCommands,
    },
    #[command(name = "set", about = "Set EDCSA/BLS key")]
    Set {
        #[command(subcommand)]
        command: SetCommands,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum ImportCommands {
    #[command(name = "bls", about = "Import a BLS key <PRIVATE_KEY>")]
    BlsImport {},
    #[command(name = "ecdsa", about = "Import a ECDSA key <PRIVATE_KEY>")]
    EcdsaImport { private_key: String, keyname: Option<String>, password: Option<String> },
}

#[derive(Parser, Debug, Clone)]
pub enum CreateCommands {
    #[command(name = "bls", about = "Create a BLS key")]
    BlsCreate {},
    #[command(name = "ecdsa", about = "Create a ECDSA key")]
    EcdsaCreate {
        #[arg(long)]
        store: bool,
        keyname: Option<String>,
        password: Option<String>,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum GetCommands {
    #[command(name = "ecdsa-private", about = "Get a ECDSA key")]
    EcdsaPrivateKey {},
    #[command(name = "ecdsa-public", about = "Get public ECDSA key")]
    EcdsaPublicKey { keyfile: String },
    #[command(name = "bls-private", about = "Get a BLS key")]
    BlsPrivateKey {},
    #[command(name = "bls-public", about = "Get public bls key")]
    BlsPublicKey {},
    #[command(name = "default-public", about = "Get the default public key")]
    GetDefaultEthAddress {},
}

#[derive(Parser, Debug, Clone)]
pub enum SetCommands {
    #[command(name = "bls", about = "Set a BLS key")]
    BlsSet {},
    #[command(name = "ecdsa", about = "Set a ECDSA key")]
    EcdsaSet { keyname: String },
}

pub async fn parse_key_subcommands(subcmd: KeyCommands, config: IvyConfig) -> Result<(), Error> {
    match subcmd {
        KeyCommands::Import { command } => {
            parse_key_import_subcommands(command, config).await?;
        }
        KeyCommands::Create { command } => {
            parse_key_create_subcommands(command, config).await?;
        }
        KeyCommands::Get { command } => {
            parse_key_get_subcommands(command, config).await?;
        }
        KeyCommands::Set { command } => {
            parse_key_set_subcommands(command, config).await?;
        }
    }
    Ok(())
}

pub async fn parse_key_import_subcommands(
    subcmd: ImportCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        ImportCommands::BlsImport {} => {}
        ImportCommands::EcdsaImport { private_key, keyname, password } => {
            let wallet = IvyWallet::from_private_key(private_key)?;
            let (keyname, pass) = get_credentials(keyname, password);
            let prv_key_path = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;
            config.default_private_keyfile = prv_key_path;
            config.store()?;
        }
    }
    Ok(())
}

pub async fn parse_key_create_subcommands(
    subcmd: CreateCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        CreateCommands::BlsCreate {} => {}
        CreateCommands::EcdsaCreate { store, keyname, password } => {
            let wallet = IvyWallet::new();
            let priv_key = wallet.to_private_key();
            println!("Private key: {:?}", priv_key);
            let addr = wallet.address();
            println!("Public Address: {:?}", addr);
            if store {
                let (keyname, pass) = get_credentials(keyname, password);
                let prv_key_path = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;
                config.default_private_keyfile = prv_key_path;
                config.store()?;
            }
        }
    }
    Ok(())
}

pub async fn parse_key_get_subcommands(
    subcmd: GetCommands,
    config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        GetCommands::BlsPrivateKey {} => {}
        GetCommands::BlsPublicKey {} => {}
        GetCommands::EcdsaPrivateKey {} => {
            let mut path = config.default_private_keyfile;
            path.set_extension("json");

            let password =
                Password::new().with_prompt("Enter a password to the private key").interact()?;
            let wallet = IvyWallet::from_keystore(path, &password)?;
            println!("Private key: {:?}", wallet.to_private_key());
        }
        GetCommands::EcdsaPublicKey { keyfile } => {
            let mut path = config.get_path().join(keyfile);
            path.set_extension("json");

            if path.exists() {
                let json = read_json_file(&path)?;
                println!("{:?}", json.get("address").expect("Cannot find public key"));
            } else {
                error!("Keyfile doesn't exist")
            }
        }
        GetCommands::GetDefaultEthAddress {} => {
            println!("Public Key: {:?}", config.default_ether_address.clone());
        }
    }
    Ok(())
}

pub async fn parse_key_set_subcommands(
    subcmd: SetCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        SetCommands::BlsSet {} => {}
        SetCommands::EcdsaSet { keyname } => {
            let mut path = config.get_path().join(keyname);
            path.set_extension("json");
            if path.exists() {
                let json = read_json_file(&path)?;
                let decoded_pub_key = extract_and_decode_pub_key(&json)?;

                config.set_private_keyfile(path);
                config.set_address(decoded_pub_key);
                config.store()?;
                println!("New default private key set")
            } else {
                println!("File doesn't exist")
            }
        }
    }
    Ok(())
}

fn read_json_file(path: &PathBuf) -> Result<Value, Error> {
    let data = fs::read_to_string(path).expect("No data in json");
    let json: Value = serde_json::from_str(&data).expect("Could not parse through json");
    Ok(json)
}

fn extract_and_decode_pub_key(json: &Value) -> Result<H160, Error> {
    let pub_key =
        json.get("address").expect("No address in json").as_str().expect("Should be a string");
    debug!("Public key: {:?}", pub_key);
    let decoded_pub_key = pub_key.parse::<H160>().expect("Should be able to convert to H160");
    Ok(decoded_pub_key)
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
    #[tokio::test]
    async fn test_import_key() {
        let test_dir = "test_import_key";
        build_test_dir(test_dir, |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());

            let result = parse_key_subcommands(
                KeyCommands::Import {
                    command: ImportCommands::EcdsaImport {
                        private_key:
                            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                                .to_string(),
                        keyname: Some("testkey".to_string()),
                        password: Some("password".to_string()),
                    },
                },
                config,
            )
            .await;

            println!("{:?}", result);
            assert!(result.is_ok());
            assert!(test_path.join("testkey.json").exists());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{:?}", config);

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
        })
        .await;
    }

    #[tokio::test]
    async fn test_create_key() {
        let test_dir = "test_create_key";
        build_test_dir(test_dir, |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());

            let result = parse_key_subcommands(
                KeyCommands::Create {
                    command: CreateCommands::EcdsaCreate {
                        store: true,
                        keyname: Some("testkey".to_string()),
                        password: Some("password".to_string()),
                    },
                },
                config,
            )
            .await;

            println!("{:?}", result);
            assert!(result.is_ok());
            assert!(test_path.join("testkey.json").exists());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{:?}", config);

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
        })
        .await;
    }
}
