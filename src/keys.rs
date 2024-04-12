use std::fs::{self, File};
use std::io::{Read, Write};

use dialoguer::Input;
use secp256k1::rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};

use pem::Pem;

use crate::config;

pub fn key_setup(private_key_string: String) {
    let secret_key: SecretKey;
    let pub_key: PublicKey; //If needed in the future

    if private_key_string.is_empty() {
        (secret_key, pub_key) = create_new_keypair();

        println! {"Please back up your private key in a safe place!"};
        println!("Private Key: {:?}", hex::encode(secret_key.secret_bytes()));
    } else {
        (secret_key, pub_key) = import_keypair(private_key_string);
    }

    let eth_address = get_eth_address(pub_key);
    println!("Address: 0x{}", eth_address);

    create_pem(secret_key);

    println!("Key created successfully!");
}

fn create_new_keypair() -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut OsRng)
}

pub fn import_keypair(secret_key: String) -> (SecretKey, PublicKey) {
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
fn create_pem(secret_key: SecretKey) {
    let key_name: String = Input::new()
        .with_prompt("Enter a name for the key")
        .interact_text()
        .expect("Error reading key name");

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
pub fn open_pem(file_path: String) -> SecretKey {
    let mut file = File::open(file_path).expect("Failed to open PEM file");

    let mut contents = Vec::new();
    file.read_to_end(&mut contents).expect("Failed to read PEM file");

    let secret_key = SecretKey::from_slice(&contents).expect("Invalid private key");

    secret_key
}
