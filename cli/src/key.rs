use aes::Aes128;
use blsful::{Bls12381G1Impl, PublicKey, SecretKey};
use clap::Parser;
use ctr::{
    cipher::{KeyIvInit, StreamCipher},
    Ctr128BE,
};
use dialoguer::{Input, Password};
use hex::{decode, encode};
use ivynet_core::{config::IvyConfig, error::IvyError, ethers::types::H160, wallet::IvyWallet};
use rand::{distributions::Alphanumeric, Rng};
use scrypt::{scrypt, Params};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};
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
    BlsImport { private_key: String, keyname: Option<String>, password: Option<String> },
    #[command(name = "ecdsa", about = "Import a ECDSA key <PRIVATE_KEY>")]
    EcdsaImport { private_key: String, keyname: Option<String>, password: Option<String> },
}

#[derive(Parser, Debug, Clone)]
pub enum CreateCommands {
    #[command(name = "bls", about = "Create a BLS key")]
    BlsCreate {
        #[arg(long)]
        store: bool,
        keyname: Option<String>,
        password: Option<String>,
    },
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
    #[command(name = "ecdsa-public", about = "Get the public ECDSA address for a specified key")]
    EcdsaPublicKey { keyname: String },
    #[command(name = "bls-private", about = "Get a BLS key")]
    BlsPrivateKey {},
    #[command(name = "bls-public", about = "Get the default public BLS key")]
    BlsPublicKey { keyname: String },
    #[command(name = "ecdsa-default", about = "Get the default ECDSA public address")]
    GetDefaultEcdsaAddress {},
    #[command(name = "bls-default", about = "Get the default BLS public address")]
    GetDefaultBlsAddress {},
}

#[derive(Parser, Debug, Clone)]
pub enum SetCommands {
    #[command(name = "bls", about = "Set a default BLS key")]
    BlsSet { keyname: String },
    #[command(name = "ecdsa", about = "Set a  default ECDSA key")]
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
            let trimmed_key = &private_key[2..];

            let hex_bytes = hex::decode(trimmed_key).expect("Invalid hex string");

            let mut array = [0u8; 32];
            array[..hex_bytes.len().min(32)].copy_from_slice(&hex_bytes[..32.min(hex_bytes.len())]);

            let sk =
                SecretKey::<Bls12381G1Impl>::from_be_bytes(&array).expect("Invalid private key");
            let (json_string, addr) = create_pub_key_and_encrypt(pass, sk);

            let file_path = config.get_bls_path().join(format!("{}.bls.key.json", keyname));

            let mut file = File::create(&file_path).expect("Couldn't create file");
            file.write_all(json_string.as_bytes()).expect("Couldn't write to json");
            println!("BLS Key has been created and saved to: {}", file_path.display());

            config.set_bls_keyfile(file_path.clone());
            config.set_bls_address(addr);
            config.store().map_err(IvyError::from)?;
        }
        ImportCommands::EcdsaImport { private_key, keyname, password } => {
            let wallet = IvyWallet::from_private_key(private_key)?;
            let (keyname, pass) = get_credentials(keyname, password);
            let prv_key_path = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;

            config.default_ecdsa_keyfile = prv_key_path;
            config.default_ecdsa_address = wallet.address();
            config.store().map_err(IvyError::from)?;
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
            if store {
                let (keyname, pass) = get_credentials(keyname, password);

                let sk = SecretKey::<Bls12381G1Impl>::new();

                let (json_string, addr) = create_pub_key_and_encrypt(pass, sk);
                let file_path = config.get_bls_path().join(format!("{}.bls.key.json", keyname));
                println!("Saved at: {:?}", file_path);

                println!("Public Address: {}", addr);

                let mut file = File::create(&file_path).expect("Couldn't create file");
                file.write_all(json_string.as_bytes()).expect("Couldn't write to json");
                println!("BLS Key has been created and saved to: {}", file_path.display());

                config.set_bls_keyfile(file_path.clone());
                config.set_bls_address(addr);
                config.store().map_err(IvyError::from)?;
            } else {
                let random_password = generate_random_string(32);
                // Generate the keypair and encrypt the private key without storing

                let sk = SecretKey::<Bls12381G1Impl>::new();

                let (_json_string, addr) = create_pub_key_and_encrypt(random_password, sk);

                // Handle the result in memory, e.g., print or return it
                println!("Generated BLS Key (in memory):");
                println!("Public Address: {}", addr);

                // Optionally, return or use the key/address in another way
            }
        }
        CreateCommands::EcdsaCreate { store, keyname, password } => {
            let wallet = IvyWallet::new();
            let priv_key = wallet.to_private_key();
            println!("Private key: {:?}", priv_key);
            let addr = wallet.address();
            println!("Public Address: {:?}", addr);
            if store {
                let (keyname, pass) = get_credentials(keyname, password);
                let prv_key_path = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;

                config.default_ecdsa_keyfile = prv_key_path;
                config.default_ecdsa_address = addr;
                config.store().map_err(IvyError::from)?;
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
        GetCommands::BlsPrivateKey {} => {
            let pass =
                Password::new().with_prompt("Enter a password to the private key").interact()?;
            //let pass = pass.trim();
            let path = config.default_bls_keyfile;

            // Read the JSON file
            let mut file = File::open(path).expect("");
            let mut json_data = String::new();
            file.read_to_string(&mut json_data).expect("json data invalid");

            // Parse the JSON data
            let parsed_json: Value = serde_json::from_str(&json_data).expect("");

            // Extract fields from JSON
            let crypto_json = &parsed_json["crypto"];
            let ciphertext_hex =
                crypto_json["ciphertext"].as_str().expect("Missing ciphertext field");
            let iv_hex = crypto_json["cipherparams"]["iv"].as_str().expect("Missing IV field");
            let salt_hex = crypto_json["kdfparams"]["salt"].as_str().expect("Missing salt field");

            // Convert hex-encoded values to bytes
            let ciphertext = decode(ciphertext_hex).expect("Failed to decode ciphertext");
            let iv = decode(iv_hex).expect("Failed to decode IV");
            let salt = decode(salt_hex).expect("Failed to decode salt");

            // Use the scrypt parameters used for encryption
            let scrypt_params = Params::new(18, 8, 1, 32).expect("Invalid parameters"); // Match the parameters used during encryption

            // Derive the key from the password
            let key = derive_key(pass.as_bytes(), &salt, &scrypt_params);

            // Decrypt the ciphertext
            let decrypted_data = decrypt_data(&ciphertext, &key, &iv);
            // Convert the decrypted data from bytes to a hex string or utf-8

            match String::from_utf8(decrypted_data) {
                Ok(decrypted_string) => {
                    println!("Decrypted BLS Private Key:\n0x{}", decrypted_string)
                }
                Err(e) => println!("Failed to convert decrypted data to UTF-8: {}", e),
            }
        }
        GetCommands::BlsPublicKey { keyname } => {
            let mut path = config.get_bls_path().join(keyname);
            path.set_extension("bls.key.json");

            if path.exists() {
                let data = fs::read_to_string(path).expect("No data in json");
                let v: Value = serde_json::from_str(&data).expect("Could not parse through json");
                println!("{}", v["pubKey"])
            } else {
                println!("No path found")
            }
        }
        GetCommands::EcdsaPrivateKey {} => {
            let mut path = config.default_ecdsa_keyfile;
            path.set_extension("json");

            if path.exists() {
                let password = Password::new()
                    .with_prompt("Enter a password to the private key")
                    .interact()?;
                let wallet = IvyWallet::from_keystore(path, &password)?;
                println!("Private key: {:?}", wallet.to_private_key());
            } else {
                println!("No path found")
            }
        }
        GetCommands::EcdsaPublicKey { keyname } => {
            let mut path = config.get_path().join(keyname);
            path.set_extension("json");

            if path.exists() {
                let json = read_json_file(&path)?;

                println!("{:?}", json.get("address").expect("Cannot find public key"));
            } else {
                error!("Keyfile doesn't exist")
            }
        }
        GetCommands::GetDefaultBlsAddress {} => {
            println!("Public Key: {:?}", config.default_bls_address.clone());
        }
        GetCommands::GetDefaultEcdsaAddress {} => {
            println!("Public Key: {:?}", config.default_ecdsa_address.clone());
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
            let mut path = config.get_bls_path().join(keyname);
            path.set_extension("bls.key.json");
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
                config.store().map_err(IvyError::from)?;
                println!("New default private key set")
            }
        }
        SetCommands::EcdsaSet { keyname } => {
            let mut path = config.get_path().join(keyname);
            path.set_extension("json");
            println!("Attempting to set key file path: {:?}", path);

            if path.exists() {
                let json = read_json_file(&path)?;
                let decoded_pub_key = extract_and_decode_pub_key(&json)?;

                config.set_ecdsa_keyfile(path);
                config.set_ecdsa_address(decoded_pub_key);
                config.store().map_err(IvyError::from)?;
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

fn decrypt_data(encrypted_data: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
    let mut cipher = Ctr128BE::<Aes128>::new(key.into(), iv.into());
    let mut buffer = encrypted_data.to_vec();
    cipher.apply_keystream(&mut buffer);
    buffer
}

// Function to derive key from password and parameters
pub fn derive_key(password: &[u8], salt: &[u8], params: &Params) -> Vec<u8> {
    let mut key = vec![0u8; 16];
    scrypt(password, salt, params, &mut key).expect("Failed to derive key");
    key
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

fn create_pub_key_and_encrypt(password: String, sk: SecretKey<Bls12381G1Impl>) -> (String, String) {
    // Generate BLS key pair

    let pk = PublicKey::<Bls12381G1Impl>::from(&sk);

    // Serialize public key to JSON
    let pub_key_json = serde_json::to_string(&pk).expect("Failed to serialize PublicKey");
    let addr = pub_key_json.trim_matches('"');

    // Convert secret key to bytes and encode as hex
    let sk_bytes = sk.to_be_bytes();
    let sk_hex = encode(sk_bytes);

    // Generate random IV and salt
    let mut rng = rand::thread_rng();
    let iv = rng.gen::<[u8; 16]>();
    let salt = rng.gen::<[u8; 32]>();

    // Derive key using scrypt
    let scrypt_params = Params::new(18, 8, 1, 32).expect("Invalid scrypt parameters");
    let key = derive_key(password.as_bytes(), &salt, &scrypt_params);

    // Encrypt the secret key
    let ciphertext = encrypt_data(sk_hex.as_bytes(), &key, &iv);

    // Generate MAC
    let mut hasher = Sha256::new();
    hasher.update(&key);
    hasher.update(&ciphertext);
    let mac = encode(hasher.finalize());

    // Construct the crypto JSON object
    let crypto_json: Value = json!({
        "cipher": "aes-128-ctr",
        "ciphertext": encode(&ciphertext),
        "cipherparams": {
            "iv": encode(iv)
        },
        "kdf": "scrypt",
        "kdfparams": {
            "dklen": 32,
            "n": 262144,
            "p": 1,
            "r": 8,
            "salt": encode(salt)
        },
        "mac": mac
    });

    // Construct the final JSON data object
    let json_data: Value = json!({
        "pubKey": addr,
        "crypto": crypto_json
    });

    // Serialize to pretty JSON string
    let json_string =
        serde_json::to_string_pretty(&json_data).expect("Failed to serialize to JSON");

    println!("Private key: {:?}", sk);

    (json_string, addr.to_string())
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

fn generate_random_string(length: usize) -> String {
    rand::thread_rng().sample_iter(&Alphanumeric).take(length).map(char::from).collect()
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
            //config.path
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
            assert_eq!(toml_data["default_ecdsa_keyfile"].as_str(), Some(private_keypath.as_str()));
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
            assert_eq!(toml_data["default_ecdsa_keyfile"].as_str(), Some(private_keypath.as_str()));
        })
        .await;
    }

    #[tokio::test]
    async fn test_create_bls_key() {
        let test_dir = "test_create_bls_key";
        build_test_dir(test_dir, |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());

            let result = parse_key_subcommands(
                KeyCommands::Create {
                    command: CreateCommands::BlsCreate {
                        store: true,
                        keyname: Some("testkey".to_string()),
                        password: Some("password".to_string()),
                    },
                },
                config,
            )
            .await;

            println!("{:?}", result);
            println!("{:?}", test_path);
            assert!(result.is_ok());
            assert!(test_path.join("testkey.json").exists());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");
            println!("{:?}", config);

            let toml_content = fs::read_to_string(test_path.join("ivy-config.toml"))
                .await
                .expect("Failed to read TOML file");
            let toml_data: toml::Value =
                toml::from_str(&toml_content).expect("Failed to parse TOML");

            // Perform assertions on TOML keys and values
            let private_keypath = format!("{}/testkey.json", test_path.to_str().unwrap());
            assert_eq!(toml_data["default_bls_keyfile"].as_str(), Some(private_keypath.as_str()))
        })
        .await;
    }
}
