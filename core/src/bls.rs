use blsful::{Bls12381G1Impl, PublicKey as BlsPublic, SecretKey as BlsSecret};
use eth_keystore::{decrypt_key, encrypt_key, KeystoreError};
use ethers::utils::hex::{decode, FromHexError};
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::io::{read_json, write_json};

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

    #[error(transparent)]
    LocalIoError(#[from] crate::io::IoError),

    #[error(transparent)]
    KeystoreError(#[from] KeystoreError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn secret(&self) -> BlsSecret<Bls12381G1Impl> {
        self.secret.clone()
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, BlsKeyError> {
        let maybe_sk = BlsSecret::from_be_bytes(bytes);

        // So so wrong
        if maybe_sk.is_some().into() {
            let secret = maybe_sk.unwrap();
            Ok(Self { secret })
        } else {
            Err(BlsKeyError::PrivateKeyInvalid)
        }
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
        // There is a chance that json is not valid for our eth_keystore (missing id and version
        // fields)
        // In order to fix this, we're making this hack
        let valid_json = read_json::<KeyfileSimplified>(&path);
        let secret_bytes = {
            if valid_json.is_ok() {
                decrypt_key(path, password)?
                    .try_into()
                    .map_err(|_| BlsKeyError::PrivateKeyInvalid)?
            } else {
                let filename = path.file_name().ok_or(BlsKeyError::KeyNotFound)?;
                let KeyfileBare { crypto } = read_json(&path)?;
                let keyfile = KeyfileSimplified { crypto, id: Uuid::new_v4().into(), version: 3 };
                let tmp_path = std::env::temp_dir().join(filename);
                write_json(&tmp_path, &keyfile)?;
                decrypt_key(tmp_path, password)?
                    .try_into()
                    .map_err(|_| BlsKeyError::PrivateKeyInvalid)?
            }
        };

        Self::from_bytes(&secret_bytes)
    }

    pub fn encrypt_and_store(
        &self,
        path: &Path,
        name: String,
        password: String,
    ) -> Result<PathBuf, BlsKeyError> {
        _ = encrypt_key(
            path,
            &mut thread_rng(),
            self.secret.to_be_bytes(),
            &password,
            Some(&name),
        )?;

        let path = path.join(name);
        let pub_key = encode_address(&self.address())?;

        let Keyfile { crypto, id, version } = read_json(&path)?;
        let keyfile = KeyfileEnriched { pub_key, crypto, id, version };

        write_json(&path, &keyfile)?;
        Ok(path)
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

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_bls_key_generation_and_import() {
        let generated_key = BlsKey::new();

        let generated_secret = generated_key.secret();
        let generated_secret_bytes = generated_secret.to_be_bytes();

        let imported_key = BlsKey::from_bytes(&generated_secret_bytes).unwrap();

        assert_eq!(generated_key, imported_key);
    }

    #[test]
    fn test_bls_key_import() {
        let imported_key = BlsKey::from_private_key(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        );

        assert!(imported_key.is_ok());
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Keyfile {
    crypto: Value,
    id: String,
    version: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyfileSimplified {
    crypto: Value,
    id: String,
    version: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyfileBare {
    crypto: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyfileEnriched {
    #[serde(rename = "pubKey")]
    pub_key: String,
    crypto: Value,
    id: String,
    version: u32,
}
