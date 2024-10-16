use std::path::Path;

use crate::error::Error;
use clap::Parser;
use dialoguer::{Input, MultiSelect, Password, Select};
use ivynet_core::{
    config::IvyConfig,
    ethers::signers::{coins_bip39::English, MnemonicBuilder},
    keychain::{Key, KeyName, KeyType, Keychain},
};
use rustix::path::Arg;

#[derive(Parser, Debug, Clone)]
pub enum KeyCommands {
    #[command(name = "import", about = "Import a ECDSA/BLS key into a keyfile")]
    Import,

    #[command(name = "create", about = "Create a ECDSA/BLS private key")]
    Create,

    #[command(name = "get", about = "Get ECDSA/BLS key information")]
    Get,
}

pub async fn parse_key_subcommands(subcmd: KeyCommands, _config: IvyConfig) -> Result<(), Error> {
    match subcmd {
        KeyCommands::Import => {
            import_key().await?;
        }
        KeyCommands::Create => {
            create_key().await?;
        }
        KeyCommands::Get => {
            get_key().await?;
        }
    }
    Ok(())
}

pub async fn import_key() -> Result<(), Error> {
    match Select::new()
        .with_prompt("Choose what type of key you would like to import")
        .items(&["BLS", "ECDSA"])
        .default(0)
        .interact()
        .expect("No key type has been chosen") {
            0 /* BLS */ => import_bls().await,
            1 /* ECDSA */ => import_ecdsa().await,
            _ => Err(Error::InvalidSelection)
        }
}

pub async fn import_bls() -> Result<(), Error> {
    // TODO: We should do a duplicate checks here as well
    match Select::new()
        .with_prompt("Would you like to import a bls keys folder, a single file or a private key?")
        .items(&["Folder", "File", "Private key"])
        .default(0)
        .interact()
        .expect("Wrong source chosen") {
            0 /* Folder */ => import_from_folder(KeyType::Bls),
            1 /* File */ => import_from_file(KeyType::Bls),
            2 /* Private key */ => import_from_key(KeyType::Bls),
            _ => Err(Error::InvalidSelection)
        }?;
    Ok(())
}

pub async fn import_ecdsa() -> Result<(), Error> {
    match Select::new()
        .with_prompt("Would you like to import a ecdsa keys folder, a single file a private key, or from mnemonic?")
        .items(&["Folder", "File", "Private key", "Mnemonic"])
        .default(0)
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
    match keychain.import_from_file(path.into(), key_type, &key_password) {
        Ok((_, key)) => {
            if key.is_type(key_type) {
                println!("Key with an address {} has been added", key.address());
            } else {
                println!(
                    "You have imported a key with address {} however it's of a different type",
                    key.address()
                );
            }
        }
        Err(e) => {
            println!("Failed to load the key {e:?}");
            return Err(Error::InvalidSelection);
        }
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

pub async fn create_key() -> Result<(), Error> {
    let key_type = match Select::new()
        .with_prompt("Choose what type of key you would like to create")
        .items(&["BLS", "ECDSA"])
        .default(0)
        .interact()
        .expect("No key type has been chosen") {
            0 /* BLS */ => Ok(KeyType::Bls),
            1 /* ECDSA */ => Ok(KeyType::Ecdsa),
            _ => Err(Error::InvalidSelection)
        }?;
    let key = create_key_of_type(key_type).await?;
    println!("Public key: {}", key.address());
    println!("Private key: {}", key.private_key_string());
    Ok(())
}

pub async fn create_key_of_type(key_type: KeyType) -> Result<Key, Error> {
    let keychain = Keychain::default();
    let key_name: String = Input::new()
        .with_prompt("Provide name for a key")
        .interact_text()
        .expect("Invalid key provided");
    let key_password = Password::new()
        .with_prompt("Provide the password to the key")
        .interact()
        .expect("Invalid password provided");
    Ok(keychain.generate(key_type, Some(&key_name), &key_password))
}

pub async fn get_key() -> Result<(), Error> {
    let keychain = Keychain::default();

    let key_list = keychain.list()?;

    if key_list.is_empty() {
        println!("You have no keys to inspect");
    } else {
        let key_index = Select::new()
            .with_prompt("Select key you want details of")
            .items(
                &key_list
                    .iter()
                    .map(|key| match key {
                        KeyName::Ecdsa(n) => format!("[ECDSA]: {n}"),
                        KeyName::Bls(n) => format!("  [BLS]: {n}"),
                    })
                    .collect::<Vec<_>>(),
            )
            .default(0)
            .interact()
            .expect("Bad selection");

        let key_password = Password::new()
            .with_prompt(format!("Provide the password to {}", key_list[key_index]))
            .interact()
            .expect("Invalid password provided");

        let key_name = key_list[key_index].clone();
        let key = keychain.load(key_name.clone(), &key_password)?;

        println!("Key name: {}", &key_name);
        println!("Path to key: {}", keychain.get_path(key_name.clone()).to_str().unwrap());
        println!("Public address: {:?}", key.address());
        println!("Private key: {}", key.private_key_string());
    }
    Ok(())
}
