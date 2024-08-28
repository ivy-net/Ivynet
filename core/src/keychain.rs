use std::{env, fs, fs::File, io::Read, path::PathBuf};

use aes::Aes128;
use ethers::{types::Address, utils::hex};
use serde_json::{json, Value};

use crate::{error::IvyError, io::write_json, wallet::IvyWallet};
use blsful::{Bls12381G1Impl, PublicKey as BlsPublic, SecretKey as BlsSecret};
use ctr::{
    cipher::{KeyIvInit, StreamCipher},
    Ctr128BE,
};
use hex::{decode, encode};
use rand::Rng;
use scrypt::{scrypt, Params};
use sha2::{Digest, Sha256};

pub enum KeyType {
    Ecdsa,
    Bls,
}

pub enum Key {
    Ecdsa(IvyWallet),
    Bls(BlsSecret<Bls12381G1Impl>),
}

pub enum KeyAddress {
    Ecdsa(Address),
    Bls(BlsPublic<Bls12381G1Impl>),
}

pub struct Keychain {
    path: PathBuf,
}

impl Default for Keychain {
    fn default() -> Self {
        Self { path: env::home_dir().unwrap() }
    }
}

impl Keychain {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn list(&self) -> Result<Vec<KeyAddress>, IvyError> {
        let paths = fs::read_dir(&self.path)?;

        let mut list = Vec::new();
        for path in paths {
            if let Ok(file) = path {
                let filename = file.file_name();
                let cmps = filename.to_str().unwrap().split(".").collect::<Vec<&str>>();
                if cmps.len() == 2 {
                    match cmps[1] {
                        "bls" => todo!(), // TODO: I have no idea how to parse a string into
                        // PublicKey struct of theirs. Stupid
                        "ecdsa" => {
                            if let Ok(address) = cmps[0].parse::<Address>() {
                                list.push(KeyAddress::Ecdsa(address));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(list)
    }

    pub fn generate(&self, key_type: KeyType, password: &str) -> Key {
        match key_type {
            KeyType::Ecdsa => Key::Ecdsa(self.ecdsa_generate(password)),
            KeyType::Bls => Key::Bls(self.bls_generate(password)),
        }
    }

    pub fn import(&self, key_type: KeyType, key: &str, password: &str) -> Result<Key, IvyError> {
        match key_type {
            KeyType::Ecdsa => Ok(Key::Ecdsa(self.ecdsa_import(key, password)?)),
            KeyType::Bls => Ok(Key::Bls(self.bls_import(key, password)?)),
        }
    }

    pub fn load(&self, address: KeyAddress, password: &str) -> Result<Key, IvyError> {
        match address {
            KeyAddress::Ecdsa(address) => Ok(Key::Ecdsa(self.ecdsa_load(address, password)?)),
            KeyAddress::Bls(address) => Ok(Key::Bls(self.bls_load(&address, password)?)),
        }
    }

    fn bls_generate(&self, password: &str) -> BlsSecret<Bls12381G1Impl> {
        let sk = BlsSecret::<Bls12381G1Impl>::new();

        self.bls_store(&sk, password);

        sk
    }

    fn bls_import(&self, key: &str, password: &str) -> Result<BlsSecret<Bls12381G1Impl>, IvyError> {
        let trimmed_key = &key[2..]; // TODO: is this trimming 0x ? If so - shouldn't we
                                     // check if it's actually something to trim?
                                     // This should definitely be a different error for this
        let hex_bytes = hex::decode(trimmed_key).map_err(|_| IvyError::InvalidAddress)?;

        let mut array = [0u8; 32];
        array[..hex_bytes.len().min(32)].copy_from_slice(&hex_bytes[..32.min(hex_bytes.len())]);

        let maybe_sk = BlsSecret::<Bls12381G1Impl>::from_be_bytes(&array);

        // This is so wrong. How they could have effed it like this?!
        if maybe_sk.is_some().into() {
            let sk = maybe_sk.unwrap();
            self.bls_store(&sk, password);
            Ok(sk)
        } else {
            Err(IvyError::InvalidAddress)
        }
    }

    // TODO: Too many expects. Needs some love
    fn bls_load(
        &self,
        address: &BlsPublic<Bls12381G1Impl>,
        password: &str,
    ) -> Result<BlsSecret<Bls12381G1Impl>, IvyError> {
        let pub_key_json = serde_json::to_string(&address).expect("Failed to serialize PublicKey");
        let addr = pub_key_json.trim_matches('"');

        let mut file = File::open(self.path.join(format!("{addr}.bls"))).expect("");
        let mut json_data = String::new();
        file.read_to_string(&mut json_data)?;
        let parsed_json: Value = serde_json::from_str(&json_data).expect("");

        // Extract fields from JSON
        let crypto_json = &parsed_json["crypto"];
        let ciphertext_hex = crypto_json["ciphertext"].as_str().expect("Missing ciphertext field");
        let iv_hex = crypto_json["cipherparams"]["iv"].as_str().expect("Missing IV field");
        let salt_hex = crypto_json["kdfparams"]["salt"].as_str().expect("Missing salt field");

        let ciphertext = decode(ciphertext_hex).expect("Failed to decode ciphertext");
        let iv = decode(iv_hex).expect("Failed to decode IV");
        let salt = decode(salt_hex).expect("Failed to decode salt");

        let scrypt_params = Params::new(18, 8, 1, 32).expect("Invalid parameters");
        let key = derive_key(password.as_bytes(), &salt, &scrypt_params);

        let decrypted_data = decrypt_data(&ciphertext, &key, &iv);

        let secret_bytes: [u8; 32] = decrypted_data.try_into().unwrap();
        let maybe_sk = BlsSecret::from_be_bytes(&secret_bytes);

        // So so wrong
        if maybe_sk.is_some().into() {
            Ok(maybe_sk.unwrap())
        } else {
            Err(IvyError::InvalidAddress)
        }
    }

    fn bls_store(&self, secret: &BlsSecret<Bls12381G1Impl>, password: &str) {
        let pk = BlsPublic::<Bls12381G1Impl>::from(secret);

        // Serialize public key to JSON
        let pub_key_json = serde_json::to_string(&pk).expect("Failed to serialize PublicKey");
        let addr = pub_key_json.trim_matches('"');

        // Convert secret key to bytes and encode as hex
        let sk_bytes = secret.to_be_bytes();
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

        _ = write_json(&self.path.join(format!("{addr}.bls")), &json_data);
    }

    fn ecdsa_generate(&self, password: &str) -> IvyWallet {
        let wallet = IvyWallet::new();
        _ = wallet.encrypt_and_store(
            &self.path,
            format!("{:?}.ecdsa", wallet.address()),
            password.to_string(),
        );
        wallet
    }

    fn ecdsa_import(&self, key: &str, password: &str) -> Result<IvyWallet, IvyError> {
        let wallet = IvyWallet::from_private_key(key.to_string())?;
        _ = wallet.encrypt_and_store(
            &self.path,
            format!("{:?}.ecdsa", wallet.address()),
            password.to_string(),
        );
        Ok(wallet)
    }

    fn ecdsa_load(&self, address: Address, password: &str) -> Result<IvyWallet, IvyError> {
        Ok(IvyWallet::from_keystore(
            self.path.join(format!("{:?}.ecdsa", address).to_string()),
            password,
        )?)
    }
}

fn encrypt_data(data: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
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

fn derive_key(password: &[u8], salt: &[u8], params: &Params) -> Vec<u8> {
    let mut key = vec![0u8; 16];
    scrypt(password, salt, params, &mut key).expect("Failed to derive key");
    key
}
