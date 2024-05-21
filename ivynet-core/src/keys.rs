use dialoguer::{Input, Password};
use ethers_core::types::Address;
use ethers_signers::{LocalWallet, Signer};
use once_cell::sync::OnceCell;
use secp256k1::rand::thread_rng;
use std::{fs, ops::DerefMut, path::PathBuf};

use crate::config::CONFIG;

pub static WALLET: OnceCell<LocalWallet> = OnceCell::new();

pub fn get_wallet() -> LocalWallet {
    WALLET.get_or_init(|| connect_wallet().unwrap()).clone()
}

pub fn create_key(
    store: bool,
    name: Option<String>,
    password: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Please back up your private key in a safe place!");
    let wallet = LocalWallet::new(&mut thread_rng());
    let priv_key = hex::encode(wallet.signer().to_bytes());
    println!("Private key: {:?}", priv_key);

    let addr = wallet.address();
    println!("Public Address: {:?}", addr);

    if store {
        encrypt_and_store(wallet, name, password)?;
    }
    Ok(())
}

pub fn import_key(
    private_key_string: String,
    name: Option<String>,
    password: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let priv_bytes = hex::decode(private_key_string)?;
    let local_wallet = LocalWallet::from_bytes(&priv_bytes)?;
    println!("Address: {:?}", local_wallet.address());

    encrypt_and_store(local_wallet, name, password)
}

pub fn encrypt_and_store(
    wallet: LocalWallet,
    name: Option<String>,
    password: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    //Find home directory and add .ivynet folder
    let mut file_path = dirs::home_dir().expect("Could not get home directory");
    file_path.push(".ivynet");

    // Create the directory if it doesn't exist
    fs::create_dir_all(&file_path)?;

    //Prompt the user to enter a password for their pkey
    let pass = if let Some(inner) = password {
        inner
    } else {
        Password::new().with_prompt("Enter a password to encrypt the private key").interact()?
    };

    //Prompt the user to enter a name for the key
    let key_name = if let Some(inner) = name {
        inner
    } else {
        Input::new().with_prompt("Enter a name for the key").interact_text()?
    };

    // ------ Private Key File ------
    let mut private_key_path: PathBuf = file_path.clone();
    println!("Private key path: {}", private_key_path.display());
    //Encrypt private key
    LocalWallet::encrypt_keystore(
        private_key_path.clone(),
        &mut thread_rng(),
        wallet.signer().to_bytes(),
        pass,
        Some(&(key_name.clone() + ".json")),
    )?;

    //Set the default private keyfile path
    private_key_path.push(format!("{}.json", key_name));

    println!("Saving keys to config path: {}", CONFIG.lock()?.get_path()?.display());

    CONFIG.lock()?.set_private_keyfile(private_key_path);

    // ------ Public Key File ------
    //Create path for pub key
    let mut pub_key_path = file_path.clone();
    pub_key_path.push(format!("{}.txt", key_name));

    //Write public key to file
    let public_key = wallet.address();
    fs::write(pub_key_path.clone(), public_key)?;

    CONFIG.lock()?.set_public_keyfile(pub_key_path);
    CONFIG.lock()?.store()?;

    println!("Key successfully stored!");

    Ok(())
}

pub fn connect_wallet() -> Result<LocalWallet, Box<dyn std::error::Error>> {
    let file_path = &CONFIG.lock()?.default_public_keyfile;
    println!("File Path: {:?}", file_path);

    let password: String =
        Password::new().with_prompt("Enter the password you used to encrypt the private key").interact()?;

    Ok(LocalWallet::decrypt_keystore(file_path, password).expect("Failed to decrypt wallet"))
}

pub fn get_stored_public_key() -> Result<Address, Box<dyn std::error::Error>> {
    let file_path = &CONFIG.lock()?.default_public_keyfile;
    let addr_vec: Vec<u8> = fs::read(file_path)?;
    let addr_bytes: [u8; 20] = addr_vec.try_into().map_err(|_| "Expected 20 bytes".to_string())?;
    Ok(Address::from(addr_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_key() {
        assert!(import_key(
            "8042944cd65953d95cb5a5d59f96a3b7e5251a05e64b98a0f0a32795c38e2247".to_string(),
            Some("test".to_string()),
            Some("jimmy".to_string())
        )
        .is_ok());
    }

    #[tokio::test]
    async fn test_get_stored_public_key() {
        let address: Address = "0xCD6908FcF7b711d5b7486F7Eb5f7F1A0504aF2c6".parse().unwrap();
        assert_eq!(get_stored_public_key().unwrap(), address);
    }
}
