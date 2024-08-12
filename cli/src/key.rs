use clap::Parser;
use dialoguer::{Input, Password};
use ivynet_core::{config::IvyConfig, error::IvyError, wallet::IvyWallet};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum KeyCommands {
    #[command(name = "import", about = "Import a ECDSA or a BLS key")]
    Import {
        #[command(subcommand)]
        command: KeyImportCommands,
    },
    #[command(name = "create", about = "Create a ECDSA or a BLS key")]
    Create {
        #[command(subcommand)]
        command: KeyCreateCommands,
    },
    #[command(name = "get", about = "Get ECDSA/BLS private/public key")]
    Get {
        #[command(subcommand)]
        command: KeyGetCommands,
    },
    #[command(name = "set", about = "Set EDCSA/BLS key")]
    Set {
        #[command(subcommand)]
        command: KeySetCommands,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum KeyImportCommands {
    #[command(name = "bls", about = "Import a BLS key <PRIVATE_KEY>")]
    BlsImport {},
    #[command(name = "ecdsa", about = "Import a ECDSA key <PRIVATE_KEY>")]
    EcdsaImport { private_key: String, keyname: Option<String>, password: Option<String> },
}

#[derive(Parser, Debug, Clone)]
pub enum KeyCreateCommands {
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
pub enum KeyGetCommands {
    #[command(name = "ecdsa-private", about = "Get a ECDSA key")]
    EcdsaPrivateKey { keyname: String },
    #[command(name = "ecdsa-public", about = "Get public ECDSA key")]
    EcdsaPublicKey {},
    #[command(name = "bls-private", about = "Get a BLS key")]
    BlsPrivateKey {},
    #[command(name = "bls-public", about = "Get public bls key")]
    BlsPublicKey {},
    #[command(name = "default-public", about = "Get the default public key")]
    GetDefaultEthAddress {},
}

#[derive(Parser, Debug, Clone)]
pub enum KeySetCommands {
    #[command(name = "bls", about = "Set a BLS key")]
    BlsSet {},
    #[command(name = "ecdsa", about = "Set a ECDSA key")]
    EcdsaSet { keyname: String },
}

pub async fn parse_key_subcommands(subcmd: KeyCommands, config: IvyConfig) -> Result<(), Error> {
    match subcmd {
        KeyCommands::Import { command } => {
            let _ = parse_key_import_subcommands(command, config).await;
        }
        KeyCommands::Create { command } => {
            let _ = parse_key_create_subcommands(command, config).await;
        }
        KeyCommands::Get { command } => {
            let _ = parse_key_get_subcommands(command, config).await;
        }
        KeyCommands::Set { command } => {
            let _ = parse_key_set_subcommands(command, config).await;
        }
    }
    Ok(())
}

pub async fn parse_key_import_subcommands(
    subcmd: KeyImportCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        KeyImportCommands::BlsImport {} => {}
        KeyImportCommands::EcdsaImport { private_key, keyname, password } => {
            let wallet = IvyWallet::from_private_key(private_key)?;
            let (keyname, pass) = get_credentials(keyname, password);
            let prv_key_path = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;
            config.default_private_keyfile = prv_key_path;
            config.store().map_err(IvyError::from)?;
        }
    }
    Ok(())
}

pub async fn parse_key_create_subcommands(
    subcmd: KeyCreateCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        KeyCreateCommands::BlsCreate {} => {}
        KeyCreateCommands::EcdsaCreate { store: _, keyname, password } => {
            let wallet = IvyWallet::new();
            let priv_key = wallet.to_private_key();
            println!("Private key: {:?}", priv_key);
            let addr = wallet.address();
            println!("Public Address: {:?}", addr);
            if true {
                // temporary
                let (keyname, pass) = get_credentials(keyname, password);
                let prv_key_path = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;
                config.default_private_keyfile = prv_key_path;
                config.store().map_err(IvyError::from)?;
            }
        }
    }
    Ok(())
}

pub async fn parse_key_get_subcommands(
    subcmd: KeyGetCommands,
    config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        KeyGetCommands::BlsPrivateKey {} => {}
        KeyGetCommands::BlsPublicKey {} => {}
        KeyGetCommands::EcdsaPrivateKey { keyname } => {
            let mut path = config.get_path().join(keyname);
            path.set_extension("json");

            let password =
                Password::new().with_prompt("Enter a password to the private key").interact()?;
            let wallet = IvyWallet::from_keystore(path, &password)?;
            println!("Private key: {:?}", wallet.to_private_key());
        }
        KeyGetCommands::EcdsaPublicKey {} => {
            let pass =
                Password::new().with_prompt("Enter a password to the private key").interact()?;

            let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), &pass)?;
            println!("Private key: {:?}", wallet.to_private_key());
            println!("{:?}", config.default_private_keyfile.clone())
        }
        KeyGetCommands::GetDefaultEthAddress {} => {
            println!("Public Key: {:?}", config.default_ether_address.clone());
        }
    }
    Ok(())
}

pub async fn parse_key_set_subcommands(
    subcmd: KeySetCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        KeySetCommands::BlsSet {} => {}
        KeySetCommands::EcdsaSet { keyname } => {
            let mut path = config.get_path().join(keyname);
            path.set_extension("json");
            if path.exists() {
                config.set_private_keyfile(path);
                println!("New default private key set")
            } else {
                println!("File doesn't exist")
            }
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
