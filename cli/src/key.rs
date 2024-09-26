use crate::error::Error;
use clap::Parser;
use ivynet_core::{
    bls::BlsKey,
    config::IvyConfig,
    error::IvyError,
    ethers::types::H160,
    keychain::{Key, KeyAddress, KeyType, Keychain},
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
    EcdsaImport { private_key: String },
    #[command(name = "bls", about = "Import a BLS private key <PRIVATE_KEY>")]
    BlsImport { private_key: String },
}

#[derive(Parser, Debug, Clone)]
pub enum CreateCommands {
    #[command(name = "ecdsa", about = "Create an ECDSA key")]
    EcdsaCreate {
        #[arg(long)]
        store: bool,
    },
    #[command(name = "bls", about = "Create a BLS key")]
    BlsCreate {
        #[arg(long)]
        store: bool,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum GetCommands {
    #[command(name = "ecdsa-private", about = "Get the default ECDSA key and its address")]
    EcdsaPrivate,
    #[command(
        name = "ecdsa-public",
        about = "Get a specified ECDSA key's public address <KEYNAME>"
    )]
    EcdsaPublicKey,
    #[command(name = "bls-private", about = "Get the default BLS key and its address")]
    BlsPrivate,
    #[command(name = "bls-public", about = "Get a specified BLS key's public address <KEYNAME>")]
    BlsPublicKey,
}

#[derive(Parser, Debug, Clone)]
pub enum SetCommands {
    #[command(name = "bls", about = "Set the default BLS key <KEYNAME>")]
    BlsSet,
    #[command(name = "ecdsa", about = "Set the default ECDSA key <KEYNAME>")]
    EcdsaSet,
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
        ImportCommands::BlsImport { private_key } => {
            let keychain = Keychain::default();
            let keyname = keychain.get_keyname(KeyType::Bls)?;
            let pass = keychain.get_password(true)?;
            let key = keychain.import(KeyType::Bls, Some(&keyname), &private_key, &pass)?;

            let addr = match key.address() {
                KeyAddress::Bls(address) => Ok(address),
                _ => Err(IvyError::IncorrectAddressError),
            }?;

            config.set_bls_keyfile(keyname.to_string());
            config.set_bls_address(addr.to_string());
            config.store()?;
        }
        ImportCommands::EcdsaImport { private_key } => {
            let keychain = Keychain::default();
            let keyname = keychain.get_keyname(KeyType::Ecdsa)?;
            let pass = keychain.get_password(true)?;
            let key = keychain.import(KeyType::Ecdsa, Some(&keyname), &private_key, &pass)?;

            println!("{:?}", key.address());
            let addr = match key.address() {
                KeyAddress::Ecdsa(address) => Ok(address),
                _ => Err(IvyError::IncorrectAddressError),
            }?;

            config.set_ecdsa_keyfile(keyname.to_string());
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
        CreateCommands::BlsCreate { store } => {
            let keychain = Keychain::default();
            if store {
                let keyname = keychain.get_keyname(KeyType::Bls)?;
                let pass = keychain.get_password(true)?;
                let key = keychain.generate(KeyType::Bls, Some(&keyname), &pass);

                let addr = match key.address() {
                    KeyAddress::Bls(address) => Ok(address),
                    _ => Err(IvyError::IncorrectAddressError),
                }?;

                config.set_bls_keyfile(keyname.to_string());
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
        CreateCommands::EcdsaCreate { store } => {
            if store {
                let keychain = Keychain::default();
                let keyname = keychain.get_keyname(KeyType::Ecdsa)?;
                let pass = keychain.get_password(true)?;

                let key = keychain.generate(KeyType::Ecdsa, Some(&keyname), &pass);

                let addr = match key.address() {
                    KeyAddress::Ecdsa(address) => Ok(address),
                    _ => Err(IvyError::IncorrectAddressError),
                }?;

                config.set_ecdsa_keyfile(keyname.to_string());
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
        GetCommands::BlsPrivate {} => {
            let keychain = Keychain::default();
            let keyname = keychain.select_key(KeyType::Bls, config.default_bls_keyfile.clone())?;
            let password = keychain.get_password(false)?;

            if let Key::Bls(key) = keychain.load(keyname, &password)? {
                println!("Private key: {:?}", key.secret());
                println!("Public Key: {:?}", config.default_bls_address.clone());
            } else {
                return Err(Error::IvyError(IvyError::IncorrectKeyTypeError))
            }
            Ok(())
        }

        GetCommands::BlsPublicKey {} => {
            let keychain = Keychain::default();
            let default_key = config.default_bls_keyfile.clone();
            let keyname = keychain.select_key(KeyType::Bls, default_key)?;
            let addr = keychain.public_address(keyname)?;
            println!("Public address: {}", addr);
            Ok(())
        }

        GetCommands::EcdsaPrivate {} => {
            let keychain = Keychain::default();
            let keyname =
                keychain.select_key(KeyType::Ecdsa, config.default_ecdsa_keyfile.clone())?;
            let password = keychain.get_password(false)?;

            if let Key::Ecdsa(key) = keychain.load(keyname, &password)? {
                println!("Private key: {:?}", key.to_private_key());
                println!("Public Key: {:?}", config.default_ecdsa_address.clone());
            } else {
                return Err(Error::IvyError(IvyError::IncorrectKeyTypeError))
            }
            Ok(())
        }

        GetCommands::EcdsaPublicKey {} => {
            let keychain = Keychain::default();
            let default_key = config.default_ecdsa_keyfile.clone();
            let keyname = keychain.select_key(KeyType::Ecdsa, default_key)?;
            let addr = keychain.public_address(keyname)?;
            println!("Public address: {}", addr);
            Ok(())
        }
    }
}

pub async fn parse_key_set_subcommands(
    subcmd: SetCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        SetCommands::BlsSet {} => {
            let keychain = Keychain::default();
            let keyname = keychain.select_key(KeyType::Bls, None)?;
            let addr = keychain.public_address(keyname.clone())?;
            config.set_bls_address(addr);
            config.set_bls_keyfile(keyname.to_string());
            config.store()?;
            println!("New default BLS key set")
        }
        SetCommands::EcdsaSet {} => {
            let keychain = Keychain::default();
            let keyname = keychain.select_key(KeyType::Ecdsa, None)?;
            let addr_string = keychain.public_address(keyname.clone())?;
            let addr = addr_string.parse::<H160>().map_err(|_| IvyError::H160Error);
            config.set_ecdsa_address(addr?);
            config.set_ecdsa_keyfile(keyname.to_string());
            config.store()?;
            println!("New default ECDSA key set")
        }
    }
    Ok(())
}
