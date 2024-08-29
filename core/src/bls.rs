use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use aes::Aes128;
use blsful::{Bls12381G1Impl, PublicKey as BlsPublic, SecretKey as BlsSecret};
use ctr::{
    cipher::{KeyIvInit, StreamCipher},
    Ctr128BE,
};
use ethers::utils::hex::{decode, encode, FromHexError};
use rand::Rng;
use scrypt::{scrypt, Params};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::io::write_json;

pub type Address = BlsPublic<Bls12381G1Impl>;

#[derive(thiserror::Error, Debug)]
pub enum BlsKeyError {
    #[error("Bad key length")]
    PrivateKeyBadLength,

    #[error("Invalid private key")]
    PrivateKeyInvalid,

    #[error("Key not found")]
    KeyNotFound,

    #[error("Malformed key file")]
    MalformedKeyFile,

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error(transparent)]
    StdIo(#[from] std::io::Error),

    #[error(transparent)]
    HexError(#[from] FromHexError),
}

#[derive(Clone, Debug)]
pub struct BlsKey {
    secret: BlsSecret<Bls12381G1Impl>,
}

impl Default for BlsKey {
    fn default() -> Self {
        Self::new()
    }
}

impl BlsKey {
    pub fn new() -> Self {
        let secret = BlsSecret::<Bls12381G1Impl>::new();
        Self { secret }
    }

    pub fn address(&self) -> Address {
        Address::from(&self.secret)
    }

    pub fn from_private_key(private_key_string: String) -> Result<Self, BlsKeyError> {
        let trimmed_key = &private_key_string[2..]; // TODO: is this trimming 0x ? If so - shouldn't we
                                                    // check if it's actually something to trim?
                                                    // This should definitely be a different error for this
        let hex_bytes = decode(trimmed_key).map_err(|_| BlsKeyError::PrivateKeyBadLength)?;

        let mut array = [0u8; 32];
        array[..hex_bytes.len().min(32)].copy_from_slice(&hex_bytes[..32.min(hex_bytes.len())]);

        let maybe_sk = BlsSecret::<Bls12381G1Impl>::from_be_bytes(&array);

        // This is so wrong. How they could have effed it like this?!
        if maybe_sk.is_some().into() {
            let secret = maybe_sk.unwrap();
            Ok(Self { secret })
        } else {
            Err(BlsKeyError::PrivateKeyInvalid)
        }
    }

    pub fn from_keystore(path: PathBuf, password: &str) -> Result<Self, BlsKeyError> {
        let mut file = File::open(path).map_err(|_| BlsKeyError::KeyNotFound)?;
        let mut json_data = String::new();
        file.read_to_string(&mut json_data)?;
        let parsed_json: Value = serde_json::from_str(&json_data)?;

        // Extract fields from JSON
        let crypto_json = &parsed_json["crypto"];
        let ciphertext_hex =
            crypto_json["ciphertext"].as_str().ok_or(BlsKeyError::MalformedKeyFile)?;
        let iv_hex =
            crypto_json["cipherparams"]["iv"].as_str().ok_or(BlsKeyError::MalformedKeyFile)?;
        let salt_hex =
            crypto_json["kdfparams"]["salt"].as_str().ok_or(BlsKeyError::MalformedKeyFile)?;

        let ciphertext = decode(ciphertext_hex)?;
        let iv = decode(iv_hex)?;
        let salt = decode(salt_hex)?;

        let scrypt_params = Params::new(18, 8, 1, 32).unwrap();
        let key = derive_key(password.as_bytes(), &salt, &scrypt_params);

        let decrypted_data = decrypt_data(&ciphertext, &key, &iv);

        let secret_bytes: [u8; 32] = decrypted_data.try_into().unwrap();
        let maybe_sk = BlsSecret::from_be_bytes(&secret_bytes);

        // So so wrong
        if maybe_sk.is_some().into() {
            let secret = maybe_sk.unwrap();
            Ok(Self { secret })
        } else {
            Err(BlsKeyError::PrivateKeyInvalid)
        }
    }

    pub fn encrypt_and_store(
        &self,
        path: &Path,
        name: String,
        password: String,
    ) -> Result<PathBuf, BlsKeyError> {
        let addr = encode_address(&self.address())?;

        // Convert secret key to bytes and encode as hex
        let sk_bytes = self.secret.to_be_bytes();
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

        let file_path = path.join(name);
        _ = write_json(&file_path, &json_data);
        Ok(file_path)
    }
}

pub fn encode_address(address: &Address) -> Result<String, BlsKeyError> {
    let pub_key_json = serde_json::to_string(&address)?;
    let addr = pub_key_json.trim_matches('"');
    Ok(addr.to_string())
}

pub fn decode_address(address: &str) -> Result<Address, BlsKeyError> {
    Ok(serde_json::from_str(&format!(r#""{address}""#))?)
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
