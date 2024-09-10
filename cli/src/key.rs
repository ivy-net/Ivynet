use aes::Aes128;
use clap::Parser;
use ctr::{
    cipher::{KeyIvInit, StreamCipher},
    Ctr128BE,
};
use dialoguer::{Input, Password};
use ivynet_core::{
    bls::BlsKey,
    config::IvyConfig,
    ethers::types::H160,
    keychain::{Key, KeyAddress, KeyName, KeyType, Keychain},
    wallet::IvyWallet,
};
use serde_json::Value;
use std::{fs, path::PathBuf};
use tracing::{debug, error};

use crate::error::Error;

#[derive(Debug)]
pub enum KeyError {
    InvalidKeyType(String),
}

#[derive(Parser, Debug, Clone)]
pub enum KeyCommands {
    #[command(name = "import", about = "Import a ECDSA/BLS private key into a keyfile")]
    Import {
        #[command(subcommand)]
        command: ImportCommands,
    },
    #[command(name = "create", about = "Create a ECDSA/BLS private key")]
    Create {
        #[command(subcommand)]
        command: CreateCommands,
    },
    #[command(name = "get", about = "Get ECDSA/BLS key information")]
    Get {
        #[command(subcommand)]
        command: GetCommands,
    },
    #[command(name = "set", about = "Set a EDCSA/BLS key as the default key")]
    Set {
        #[command(subcommand)]
        command: SetCommands,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum ImportCommands {
    #[command(name = "ecdsa", about = "Import a ECDSA private key <PRIVATE_KEY>")]
    EcdsaImport { private_key: String, keyname: Option<String>, password: Option<String> },
    #[command(name = "bls", about = "Import a BLS private key <PRIVATE_KEY>")]
    BlsImport { private_key: String, keyname: Option<String>, password: Option<String> },
}

#[derive(Parser, Debug, Clone)]
pub enum CreateCommands {
    #[command(name = "ecdsa", about = "Create an ECDSA key")]
    EcdsaCreate {
        #[arg(long)]
        store: bool,
        keyname: Option<String>,
        password: Option<String>,
    },
    #[command(name = "bls", about = "Create a BLS key")]
    BlsCreate {
        #[arg(long)]
        store: bool,
        keyname: Option<String>,
        password: Option<String>,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum GetCommands {
    #[command(name = "ecdsa-private", about = "Get the default ECDSA key and its address")]
    EcdsaPrivate { keyname: Option<String> },
    #[command(
        name = "ecdsa-public",
        about = "Get a specified ECDSA key's public address <KEYNAME>"
    )]
    EcdsaPublicKey { keyname: Option<String> },
    #[command(name = "bls-private", about = "Get the default BLS key and its address")]
    BlsPrivate { keyname: Option<String> },
    #[command(name = "bls-public", about = "Get a specified BLS key's public address <KEYNAME>")]
    BlsPublicKey { keyname: Option<String> },
}

#[derive(Parser, Debug, Clone)]
pub enum SetCommands {
    #[command(name = "bls", about = "Set the default BLS key <KEYNAME>")]
    BlsSet { keyname: String },
    #[command(name = "ecdsa", about = "Set the default ECDSA key <KEYNAME>")]
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
        ImportCommands::BlsImport { private_key, keyname, password } => {
            let (keyname, pass) = get_credentials(keyname, password);
            let keychain = Keychain::default();
            let key = keychain.import(KeyType::Bls, Some(&keyname), &private_key, &pass)?;

            let addr = match key.address() {
                KeyAddress::Bls(address) => Ok(address),
                _ => Err(KeyError::InvalidKeyType("Not a valid BLS public address".to_string())),
            }
            .expect("");
            let path = config.get_key_path().join(format!("{}.bls.json", keyname));

            config.set_bls_keyfile(path);
            config.set_bls_address(addr.to_string());
            config.store()?;
        }
        ImportCommands::EcdsaImport { private_key, keyname, password } => {
            let (keyname, pass) = get_credentials(keyname, password);
            let keychain = Keychain::default();
            let key = keychain.import(KeyType::Ecdsa, Some(&keyname), &private_key, &pass)?;
            println!("{:?}", key.address());
            let addr = match key.address() {
                KeyAddress::Ecdsa(address) => Ok(address),
                _ => Err(KeyError::InvalidKeyType("Not a valid ECDSA public address".to_string())),
            }
            .expect("");
            let path = config.get_key_path().join(format!("{}.ecdsa.json", keyname));

            config.set_ecdsa_keyfile(path);
            config.set_ecdsa_address(addr);
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
        CreateCommands::BlsCreate { store, keyname, password } => {
            let keychain = Keychain::default();
            if store {
                let (keyname, pass) = get_credentials(keyname, password);
                let key = keychain.generate(KeyType::Bls, Some(&keyname), &pass);

                let addr = match key.address() {
                    KeyAddress::Bls(address) => Ok(address),
                    _ => Err(KeyError::InvalidKeyType("Not a valid BLS address".to_string())),
                }
                .expect("Invalid bls address");
                let path = config.get_key_path().join(format!("{}.bls.json", keyname));

                config.set_bls_keyfile(path);
                config.set_bls_address(addr.to_string());
                config.store()?;

                println!("Public key: {:?}", addr);
                println!("Private key: {:?}", key);
            } else {
                let key = BlsKey::new();
                let addr = key.address();
                println!("Public key: {:?}", addr);
                println!("Private key: {:?}", key.secret());
            }
        }
        CreateCommands::EcdsaCreate { store, keyname, password } => {
            if store {
                let keychain = Keychain::default();
                let (keyname, pass) = get_credentials(keyname, password);
                let key = keychain.generate(KeyType::Ecdsa, Some(&keyname), &pass);

                let addr = match key.address() {
                    KeyAddress::Ecdsa(address) => Ok(address),
                    _ => Err(KeyError::InvalidKeyType("Not a valid BLS address".to_string())),
                }
                .expect("Invalid Ecdsa address");
                let path = config.get_key_path().join(format!("{}.ecdsa.json", keyname));

                config.set_ecdsa_keyfile(path);
                config.set_ecdsa_address(addr);
                config.store()?;

                println!("Public key: {:?}", addr);
                println!("Private key: {:?}", key);
            } else {
                let key = IvyWallet::new();
                let addr = key.address();
                println!("Public key: {:?}", addr);
                println!("Private key: {:?}", key.to_private_key());
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
        GetCommands::BlsPrivate { keyname } => {
            let keyname = keyname.unwrap_or_else(|| {
                let mut keyname = None;
                let path = config.default_bls_keyfile.clone();
                if let Some(file_stem) = path.file_stem() {
                    if let Some(stem_str) = file_stem.to_str() {
                        if let Some(name) = stem_str.split('.').next() {
                            keyname = Some(name.to_string());
                        }
                    }
                }
                keyname.unwrap_or_default()
            });

            let password = Password::new()
                .with_prompt("Enter a password to the private key")
                .interact()
                .expect("");

            let keychain = Keychain::default();
            if let Key::Bls(key) = keychain.load(KeyName::Bls(keyname), &password)? {
                println!("Private key: {:?}", key.secret());
                println!("Public Key: {:?}", config.default_bls_address.clone())
            } else {
                //return Err(KeyError::InvalidKeyType("Not a BLS key".to_string()))
                //The if let does already throw error:
                //Error: IO: No such file or directory (os error 2)
            }
        }

        GetCommands::BlsPublicKey { keyname } => {
            let mut path;
            match keyname {
                Some(keyname) => {
                    path = config.get_key_path().join(keyname);
                    path.set_extension("bls.json");
                }
                None => {
                    println!("{:?}", config.default_bls_address);
                    return Ok(());
                }
            }
            if path.exists() {
                let data = fs::read_to_string(path).expect("No data in json");
                let v: Value = serde_json::from_str(&data).expect("Could not parse through json");
                println!("{}", v["pubKey"])
            } else {
                println!("No path found")
            }
        }
        GetCommands::EcdsaPrivate { keyname } => {
            let keyname = keyname.unwrap_or_else(|| {
                let mut keyname = None;
                let path = config.default_ecdsa_keyfile.clone();

                if let Some(file_stem) = path.file_stem() {
                    if let Some(stem_str) = file_stem.to_str() {
                        if let Some(name) = stem_str.split('.').next() {
                            keyname = Some(name.to_string());
                        }
                    }
                }

                keyname.unwrap_or_default()
            });

            let password = Password::new()
                .with_prompt("Enter a password to the private key")
                .interact()
                .expect("");

            let keychain = Keychain::default();
            if let Key::Ecdsa(key) = keychain.load(KeyName::Ecdsa(keyname), &password)? {
                println!("Private key: {:?}", key.to_private_key());
                println!("Public Key: {:?}", config.default_ecdsa_address.clone())
            } else {
                //return Err(KeyError::InvalidKeyType("Not a BLS key".to_string()))
                //The if let does already throw error:
                //Error: IO: No such file or directory (os error 2)
            }
        }
        GetCommands::EcdsaPublicKey { keyname } => {
            let mut path;
            match keyname {
                Some(keyname) => {
                    path = config.get_path().join(keyname);
                    path.set_extension("ecdsa.json");
                }
                None => {
                    println!("{:?}", config.default_ecdsa_address);
                    return Ok(());
                }
            }

            if path.exists() {
                let json = read_json_file(&path).expect("");
                println!("{:?}", json.get("address").expect("Cannot find public key"));
            } else {
                error!("{:?} :: Keyfile doesn't exist", path)
            }
        }
    }
    Ok(())
}

pub async fn parse_key_set_subcommands(
    subcmd: SetCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        SetCommands::BlsSet { keyname } => {
            let mut path = config.get_key_path().join(keyname);
            path.set_extension("bls.json");
            println!("Attempting to set key file path: {:?}", path);
            if path.exists() {
                let json = read_json_file(&path)?;
                let pub_key = json
                    .get("pubKey")
                    .expect("No address in json")
                    .as_str()
                    .expect("Should be a string");

                config.set_bls_keyfile(path);
                config.set_bls_address(pub_key.to_string());
                config.store()?;
                println!("New default private key set")
            }
        }
        SetCommands::EcdsaSet { keyname } => {
            let mut path = config.get_path().join(keyname);
            path.set_extension("ecdsa.json");
            println!("Attempting to set key file path: {:?}", path);

            if path.exists() {
                let json = read_json_file(&path)?;
                let decoded_pub_key = extract_and_decode_pub_key(&json)?;

                config.set_ecdsa_keyfile(path);
                config.set_ecdsa_address(decoded_pub_key);
                config.store()?;
                println!("New default private key set")
            } else {
                println!("File doesn't exist at path: {:?}", path);
            }
        }
    }
    Ok(())
}

pub fn encrypt_data(data: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
    let mut cipher = Ctr128BE::<Aes128>::new(key.into(), iv.into());
    let mut buffer = data.to_vec();
    cipher.apply_keystream(&mut buffer);
    buffer
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
    use std::{future::Future, path::PathBuf, str};
    use tokio::fs;

    pub async fn build_test_dir<F, Fut, T>(test_dir: &str, test_logic: F) -> T
    where
        F: FnOnce(PathBuf) -> Fut,
        Fut: Future<Output = T>,
    {
        let test_path = std::env::current_dir().unwrap().join(format!("{}", test_dir));
        fs::create_dir_all(&test_path).await.expect("Failed to create testing_temp directory");
        let result = test_logic(test_path.clone()).await;
        fs::remove_dir_all(test_path).await.expect("Failed to delete testing_temp directory");

        result
    }
    #[tokio::test]
    async fn test_import_ecdsa_key() {
        let test_dir = "test_import_key";
        build_test_dir(test_dir, |test_path| async move {
            let mut config = IvyConfig::new_at_path(test_path.clone());
            config.set_keypath(test_path.clone());
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
                config.clone(),
            )
            .await;

            println!("{:?}", result);
            assert!(result.is_ok());
            println!("{:?}", test_path);
            assert!(test_path.join("testkey.ecdsa.json").exists());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{:?}", config);

            let toml_content = fs::read_to_string(test_path.join("ivy-config.toml"))
                .await
                .expect("Failed to read TOML file");
            let toml_data: toml::Value =
                toml::from_str(&toml_content).expect("Failed to parse TOML");

            // Perform assertions on TOML keys and values
            let private_keypath = format!("{}/testkey.ecdsa.json", test_path.to_str().unwrap());
            assert_eq!(toml_data["default_ecdsa_keyfile"].as_str(), Some(private_keypath.as_str()));
        })
        .await;
    }

    #[tokio::test]
    async fn test_create_ecdsa_key() {
        let test_dir = "test_create_key";
        build_test_dir(test_dir, |test_path| async move {
            let mut config = IvyConfig::new_at_path(test_path.clone());
            config.set_keypath(test_path.clone());
            let result = parse_key_subcommands(
                KeyCommands::Create {
                    command: CreateCommands::EcdsaCreate {
                        store: true,
                        keyname: Some("testkey".to_string()),
                        password: Some("password".to_string()),
                    },
                },
                config.clone(),
            )
            .await;

            println!("{:?}", result);
            assert!(result.is_ok());
            assert!(test_path.join("testkey.ecdsa.json").exists());

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
            let private_keypath = format!("{}/testkey.ecdsa.json", test_path.to_str().unwrap());
            assert_eq!(toml_data["default_ecdsa_keyfile"].as_str(), Some(private_keypath.as_str()));
        })
        .await;
    }
    #[tokio::test]
    async fn test_get_ecdsa_private_key() {
        let test_dir = "test_get_ecdsa_private_key";
        build_test_dir(test_dir, |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());

            let _ = parse_key_subcommands(
                KeyCommands::Create {
                    command: CreateCommands::EcdsaCreate {
                        store: true,
                        keyname: Some("testkey".to_string()),
                        password: Some("password".to_string()),
                    },
                },
                config.clone(),
            )
            .await;
            let result = parse_key_subcommands(
                KeyCommands::Get { command: GetCommands::EcdsaPrivate { keyname: None } },
                config,
            )
            .await;

            println!("{:?}", result);
            assert!(result.is_ok());
        })
        .await;
    }
    #[tokio::test]
    async fn test_get_public_key() {
        let test_dir = "test_get_public_key";
        build_test_dir(test_dir, |test_path| async move {
            let mut config = IvyConfig::new_at_path(test_path.clone());
            config.set_keypath(test_path.clone());
            let create_result = parse_key_subcommands(
                KeyCommands::Create {
                    command: CreateCommands::EcdsaCreate {
                        store: true,
                        keyname: Some("testkey".to_string()),
                        password: Some("password".to_string()),
                    },
                },
                config.clone(),
            )
            .await;

            assert!(create_result.is_ok());

            let get_result = parse_key_subcommands(
                KeyCommands::Get {
                    command: GetCommands::EcdsaPublicKey { keyname: Some("testkey".to_string()) },
                },
                config.clone(),
            )
            .await;

            assert!(get_result.is_ok());

            let keyfile_path = test_path.join("testkey.ecdsa.json");
            assert!(keyfile_path.exists());

            let json = read_json_file(&keyfile_path).expect("Failed to read keyfile");
            let address = json
                .get("address")
                .expect("Address field missing in keyfile")
                .as_str()
                .expect("Address should be a string");

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{:?}", config);

            assert_eq!(
                address.parse::<H160>().expect("Should be able to convert to H160"),
                config.default_ecdsa_address,
                "The public key address does not match the address in the config file"
            );
        })
        .await;
    }
    #[tokio::test]
    async fn test_import_bls_key() {
        let test_dir = "testbls_import";
        build_test_dir(test_dir, |test_path| async move {
            let mut config = IvyConfig::new_at_path(test_path.clone());
            config.set_keypath(test_path.clone());
            let result = parse_key_subcommands(
                KeyCommands::Import {
                    command: ImportCommands::BlsImport {
                        private_key:
                            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                                .to_string(),
                        keyname: Some("testblsimport".to_string()),
                        password: Some("password".to_string()),
                    },
                },
                config,
            )
            .await;

            println!("{:?}", result);
            assert!(result.is_ok());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{:?}", config);

            let toml_content = fs::read_to_string(test_path.join("ivy-config.toml"))
                .await
                .expect("Failed to read TOML file");
            let toml_data: toml::Value =
                toml::from_str(&toml_content).expect("Failed to parse TOML");

            let private_keypath = format!(
                "{}/testblsimport.bls.json",
                test_path.to_str().expect("Can't cast to string")
            );
            assert_eq!(toml_data["default_bls_keyfile"].as_str(), Some(private_keypath.as_str()));
        })
        .await;
    }
    #[tokio::test]
    async fn test_create_bls_key() {
        let test_dir = "testbls_key";
        build_test_dir(test_dir, |test_path| async move {
            let mut config = IvyConfig::new_at_path(test_path.clone());
            config.set_keypath(test_path.clone());
            let result = parse_key_subcommands(
                KeyCommands::Create {
                    command: CreateCommands::BlsCreate {
                        store: true,
                        keyname: Some("testblskey".to_string()),
                        password: Some("password".to_string()),
                    },
                },
                config,
            )
            .await;

            println!("{:?}", result);
            assert!(result.is_ok());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{:?}", config);

            let toml_content = fs::read_to_string(test_path.join("ivy-config.toml"))
                .await
                .expect("Failed to read TOML file");
            let toml_data: toml::Value =
                toml::from_str(&toml_content).expect("Failed to parse TOML");

            // Perform assertions on TOML keys and values
            let private_keypath = format!(
                "{}/testblskey.bls.json",
                test_path.to_str().expect("Can't cast to string")
            );
            assert_eq!(toml_data["default_bls_keyfile"].as_str(), Some(private_keypath.as_str()));
        })
        .await;
    }
}
