use dialoguer::Input;
use ethers_core::types::{Address, H160, U256};
use ethers_signers::LocalWallet;
use pem::Pem;
use secp256k1::rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};
use std::fs::{self, File};
use std::io::{Read, Write};

use crate::config;

lazy_static::lazy_static! {
    pub static ref WALLET: LocalWallet = connect_wallet();
}

pub fn create_key(store: bool, name: Option<String>) {
    let secp = Secp256k1::new();
    let (secret_key, pub_key) = secp.generate_keypair(&mut OsRng);

    println! {"Please back up your private key in a safe place!"};
    println!("Private Key: {:?}", hex::encode(secret_key.secret_bytes()));

    let eth_address = get_eth_address(pub_key);
    println!("Address: 0x{}", eth_address);

    if store {
        create_pem(secret_key, name);
    }
}

pub fn import_key(private_key_string: String, name: Option<String>) {
    let (secret_key, pub_key) = import_keypair(private_key_string);

    let eth_address = get_eth_address(pub_key);
    println!("Address: 0x{}", eth_address);

    create_pem(secret_key, name);
}

fn import_keypair(secret_key: String) -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();

    let hex_secret_key = hex::decode(secret_key).expect("Expected a valid hex string");
    let secret_key = SecretKey::from_slice(&hex_secret_key).expect("Invalid private key");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    (secret_key, public_key)
}

// Gets the Ethereum public address, the last 20 bytes, from the last 64 bytes of the public key
pub fn get_eth_address(pub_key: PublicKey) -> String {
    let pub_uncomp = pub_key.serialize_uncompressed();
    let pub_slice = &pub_uncomp[1..];
    // println!("Public Key: {:?}", hex::encode(pub_slice));

    let mut hasher = Keccak256::new();
    hasher.update(pub_slice);
    let result = hasher.finalize();

    hex::encode(&result[12..])
}

pub fn get_eth_address_from_secret(secret_key: SecretKey) -> String {
    let secp = Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    get_eth_address(public_key)
}

// Create a PEM file with the private key and save it password protected
fn create_pem(secret_key: SecretKey, name: Option<String>) {
    let key_name: String;
    if name.is_none() {
        key_name = Input::new()
            .with_prompt("Enter a name for the key")
            .interact_text()
            .expect("Error reading key name");
    } else {
        key_name = name.unwrap();
    }

    //TODO: Enable password protection for the PEM file
    // let password: String = Password::new()
    //     .with_prompt("Enter a password to encrypt the private key")
    //     .interact()
    //     .expect("Error reading password");

    let pem = Pem::new(key_name.clone(), secret_key.secret_bytes());

    let mut file_path = dirs::home_dir().expect("Could not get home directory");
    file_path.push(".ivynet");

    // Create the directory if it doesn't exist
    fs::create_dir_all(&file_path).expect("Failed to create directory");

    file_path.push(format!("{}.pem", key_name));

    // Build the file
    let mut file = File::create(file_path.as_path()).expect("Failed to create PEM file");
    file.write_all(pem.contents()).expect("Failed to write PEM to file");

    config::set_default_keyfile(file_path.to_str().unwrap().to_string());

    println!("PEM file created successfully!");
}

// Open a PEM file and return the private key
fn open_pem(file_path: String) -> SecretKey {
    let mut file = File::open(file_path).expect("Failed to open PEM file");

    let mut contents = Vec::new();
    file.read_to_end(&mut contents).expect("Failed to read PEM file");

    let secret_key = SecretKey::from_slice(&contents).expect("Invalid private key");

    secret_key
}

pub fn get_secret_from_config() -> SecretKey {
    let config = config::load_config();
    let keyfile = config.default_keyfile;

    open_pem(keyfile)
}

pub fn get_keystring() -> String {
    let secret_key = get_secret_from_config();
    hex::encode(secret_key.secret_bytes())
}

pub fn connect_wallet() -> LocalWallet {
    get_keystring()
        .parse::<LocalWallet>()
        .expect("Could not connect to wallet")
}
