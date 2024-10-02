use std::path::Path;

use crate::error::Error;
use clap::Parser;
use dialoguer::{Input, MultiSelect, Password, Select};
use ivynet_core::{
    bls::BlsKey,
    config::IvyConfig,
    error::IvyError,
    ethers::signers::{coins_bip39::English, MnemonicBuilder},
    keychain::{Key, KeyAddress, KeyType, Keychain},
    wallet::IvyWallet,
};
use rustix::path::Arg;

#[derive(Parser, Debug, Clone)]
pub enum KeyCommands {
    #[command(name = "import", about = "Import a ECDSA/BLS key into a keyfile")]
    Import,

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
        KeyCommands::Import => {
            import_key().await?;
        }
        KeyCommands::Create { command } => {
            parse_key_create_subcommands(command, config).await?;
        }
        KeyCommands::Get { command } => {
            parse_key_get_subcommands(command, config).await?;
        }
    }
    Ok(())
}

pub async fn import_key() -> Result<(), Error> {
    match Select::new()
        .with_prompt("Choose what type of key you would like to import")
        .items(&["BLS", "ECDSA"])
        .interact()
        .expect("No key type has been chosen") {
            0 /* BLS */ => import_bls().await,
            1 /* ECDSA */ => import_ecdsa().await,
            _ => Err(Error::InvalidSelection)
        }
}

async fn import_bls() -> Result<(), Error> {
    // TODO: We should do a duplicate checks here as well
    match Select::new()
        .with_prompt("Would you like to import a bls keys folder, a single file or a private key?")
        .items(&["Folder", "File", "Private key"])
        .interact()
        .expect("Wrong source chosen") {
            0 /* Folder */ => import_from_folder(KeyType::Bls),
            1 /* File */ => import_from_file(KeyType::Bls),
            2 /* Private key */ => import_from_key(KeyType::Bls),
            _ => Err(Error::InvalidSelection)
        }?;
    Ok(())
}

async fn import_ecdsa() -> Result<(), Error> {
    match Select::new()
        .with_prompt("Would you like to import a ecdsa keys folder, a single file a private key, or from mnemonic?")
        .items(&["Folder", "File", "Private key", "Mnemonic"])
        .interact()
        .expect("Wrong source chosen") {
            0 /* Folder */ => import_from_folder(KeyType::Ecdsa),
            1 /* File */ => import_from_file(KeyType::Ecdsa),
            2 /* Private key */ => import_from_key(KeyType::Ecdsa),
            3 /* Mnemonic */ => import_from_mnemonic(),
            _ => Err(Error::InvalidSelection)
        }
}

fn import_from_mnemonic() -> Result<(), Error> {
    let keychain = Keychain::default();
    let mnemonic: String =
        Input::new().with_prompt("Provide mnemonic").interact().expect("No mnemonic provided");
    let idx: u32 = Input::new()
        .with_prompt("Provide address index")
        .interact()
        .expect("Invalid index provided");
    if let Ok(wallet) = MnemonicBuilder::<English>::default()
        .phrase(mnemonic.as_str())
        .index(idx)
        .expect("Invalid index")
        .build()
    {
        let private_key = format!(
            "0x{}",
            data_encoding::HEXLOWER_PERMISSIVE.encode(wallet.signer().to_bytes().as_slice()),
        );
        let key_name: String = Input::new()
            .with_prompt("Provide name for a key")
            .interact_text()
            .expect("Invalid key provided");
        let key_password = Password::new()
            .with_prompt("Provide the password to the key")
            .interact()
            .expect("Invalid password provided");
        if let Ok(key) =
            keychain.import(KeyType::Ecdsa, Some(&key_name), private_key.as_ref(), &key_password)
        {
            println!("Key with an address {} has been added", key.address());
        } else {
            println!("Provided private key is invalid.");
            return Err(Error::InvalidSelection);
        }

        Ok(())
    } else {
        println!("Wrong mnemonic provided");
        Err(Error::InvalidSelection)
    }
}

fn import_from_folder(key_type: KeyType) -> Result<(), Error> {
    let folder_path_str: String =
        Input::new().with_prompt("Enter folder path").interact().expect("No path provided");
    // checking if path exists
    let folder_path = Path::new(&folder_path_str);
    if folder_path.exists() {
        let files = folder_path
            .read_dir()
            .expect("Path is not a directory")
            .filter_map(|f| {
                if let Ok(f) = f {
                    f.path().into_os_string().into_string().ok()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for idx in MultiSelect::new()
            .with_prompt(
                "Choose files to import selecting them using spacebar and accepting with ENTER",
            )
            .items(&files)
            .interact()
            .expect("Bad items selected")
        {
            _ = import_from(key_type, files.get(idx).expect("File should exist on the list"));
        }

        Ok(())
    } else {
        Err(Error::InvalidSelection)
    }
}

fn import_from_file(key_type: KeyType) -> Result<(), Error> {
    let file_path: String = Input::new()
        .with_prompt("Provide path to the key file")
        .interact_text()
        .expect("Invalid path");

    // We need to check if that file exists at least
    let path = Path::new(&file_path);
    if path.exists() {
        import_from(
            key_type,
            path.into_c_str().expect("Uparsable path").to_str().expect("Unparsable string path"),
        )?;
    } else {
        println!("File does not exist at path {file_path}");
    }
    Ok(())
}

fn import_from(key_type: KeyType, path: &str) -> Result<(), Error> {
    let keychain = Keychain::default();
    let key_password = Password::new()
        .with_prompt("Provide the password to the key")
        .interact()
        .expect("Invalid password provided");
    if let Ok((_, key)) = keychain.import_from_file(path.into(), key_type, &key_password) {
        if key.is_type(key_type) {
            println!("Key with an address {} has been added", key.address());
        } else {
            println!(
                "You have imported a key with address {} however it's of a different type",
                key.address()
            );
        }
    } else {
        println!("Failed to load the key");
        return Err(Error::InvalidSelection);
    }
    Ok(())
}

fn import_from_key(key_type: KeyType) -> Result<(), Error> {
    let keychain = Keychain::default();
    let key_name: String = Input::new()
        .with_prompt("Provide name for a key")
        .interact_text()
        .expect("Invalid key provided");
    let key_password = Password::new()
        .with_prompt("Provide the password to the key")
        .interact()
        .expect("Invalid password provided");
    let mut imported = false;
    while !imported {
        if let Ok(possible_key) =
            Input::<String>::new().with_prompt("Enter your private key").interact_text()
        {
            if let Ok(key) =
                keychain.import(key_type, Some(&key_name), possible_key.as_ref(), &key_password)
            {
                println!("Key with an address {} has been added", key.address());
                imported = true;
            } else {
                println!("Provided private key is invalid, try again.");
            }
        }
    }
    Ok(())
}

pub async fn parse_key_create_subcommands(
    subcmd: CreateCommands,
    mut _config: IvyConfig,
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
    _config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        GetCommands::BlsPrivate {} => {
            let keychain = Keychain::default();
            let keyname = keychain.select_key(KeyType::Bls)?;

            let password = Password::new()
                .with_prompt("Enter a password to the private key")
                .interact()
                .expect("No password provided");

            if let Key::Bls(key) = keychain.load(keyname, &password)? {
                println!("Private key: {:?}", key.secret());
                println!("Public Key: {:?}", key.address());
            } else {
                return Err(Error::IvyError(IvyError::IncorrectKeyTypeError));
            }
            Ok(())
        }

        GetCommands::BlsPublicKey {} => {
            let keychain = Keychain::default();
            let keyname = keychain.select_key(KeyType::Bls)?;
            let addr = keychain.public_address(keyname)?;
            println!("Public address: {}", addr);
            Ok(())
        }

        GetCommands::EcdsaPrivate {} => {
            let keychain = Keychain::default();
            let keyname = keychain.select_key(KeyType::Ecdsa)?;

            let password = Password::new()
                .with_prompt("Enter a password to the private key")
                .interact()
                .expect("No password provided");

            if let Key::Ecdsa(key) = keychain.load(keyname, &password)? {
                println!("Private key: {:?}", key.to_private_key());
                println!("Public Key: {:?}", key.address());
            } else {
                return Err(Error::IvyError(IvyError::IncorrectKeyTypeError));
            }
            Ok(())
        }

        GetCommands::EcdsaPublicKey {} => {
            let keychain = Keychain::default();
            let keyname = keychain.select_key(KeyType::Ecdsa)?;
            let addr = keychain.public_address(keyname)?;
            println!("Public address: {}", addr);
            Ok(())
        }
    }
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
