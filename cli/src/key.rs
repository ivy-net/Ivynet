use crate::error::Error;
use clap::Parser;
use dialoguer::{Input, Password};
use ivynet_core::{
    bls::BlsKey,
    config::IvyConfig,
    error::IvyError,
    ethers::types::H160,
    keychain::{Key, KeyAddress, KeyName, KeyType, Keychain},
    wallet::IvyWallet,
};

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
                _ => Err(IvyError::IncorrectAddressError),
            }?;

            let path = keychain.get_path(KeyName::Bls(keyname));

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
                _ => Err(IvyError::IncorrectAddressError),
            }?;
            let path = keychain.get_path(KeyName::Ecdsa(keyname));

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
                    _ => Err(IvyError::IncorrectAddressError),
                }?;
                let path = keychain.get_path(KeyName::Bls(keyname));

                config.set_bls_keyfile(path);
                config.set_bls_address(addr.to_string());
                config.store()?;

                println!("Public key: {:?}", addr);
                println!("Private key: {:?}", key.private_key_string());
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
                    _ => Err(IvyError::IncorrectAddressError),
                }?;
                let path = keychain.get_path(KeyName::Ecdsa(keyname));

                config.set_ecdsa_keyfile(path);
                config.set_ecdsa_address(addr);
                config.store()?;

                println!("Public key: {:?}", addr);
                println!("Private key: 0x{}", key.private_key_string());
            } else {
                let key = IvyWallet::new();
                let addr = key.address();
                println!("Public key: {:?}", addr);
                println!("Private key: 0x{}", key.to_private_key());
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
                .expect("No password provided");

            let keychain = Keychain::default();
            if let Key::Bls(key) = keychain.load(KeyName::Bls(keyname), &password)? {
                println!("Private key: {:?}", key.secret());
                println!("Public Key: {:?}", config.default_bls_address.clone());
            } else {
                return Err(Error::IvyError(IvyError::IncorrectKeyTypeError))
            }
            Ok(())
        }

        GetCommands::BlsPublicKey { keyname } => {
            match keyname {
                Some(keyname) => {
                    let keychain = Keychain::default();
                    let addr = keychain.public_address(KeyName::Bls(keyname))?;
                    println!("{}", addr)
                }
                None => {
                    println!("{:?}", config.default_bls_address)
                }
            }
            Ok(())
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
                .expect("No password provided");

            let keychain = Keychain::default();
            if let Key::Ecdsa(key) = keychain.load(KeyName::Ecdsa(keyname), &password)? {
                println!("Private key: {:?}", key.to_private_key());
                println!("Public Key: {:?}", config.default_ecdsa_address.clone());
            } else {
                return Err(Error::IvyError(IvyError::IncorrectKeyTypeError))
            }
            Ok(())
        }
        GetCommands::EcdsaPublicKey { keyname } => {
            match keyname {
                Some(keyname) => {
                    let keychain = Keychain::default();
                    let addr = keychain.public_address(KeyName::Ecdsa(keyname))?;
                    println!("{}", addr)
                }
                None => {
                    println!("{:?}", config.default_ecdsa_address)
                }
            }
            Ok(())
        }
    }
}

pub async fn parse_key_set_subcommands(
    subcmd: SetCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        SetCommands::BlsSet { keyname } => {
            let keychain = Keychain::default();
            let addr = keychain.public_address(KeyName::Bls(keyname.clone()))?;
            let path = keychain.get_path(KeyName::Bls(keyname));
            config.set_bls_address(addr);
            config.set_bls_keyfile(path);
            config.store()?;
            println!("New default private key set")
        }
        SetCommands::EcdsaSet { keyname } => {
            let keychain = Keychain::default();
            let addr_string = keychain.public_address(KeyName::Ecdsa(keyname.clone()))?;
            let addr = addr_string.parse::<H160>().map_err(|_| IvyError::H160Error);
            let path = keychain.get_path(KeyName::Ecdsa(keyname));
            config.set_ecdsa_address(addr?);
            config.set_ecdsa_keyfile(path);
            config.store()?;
            println!("New default private key set")
        }
    }
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
