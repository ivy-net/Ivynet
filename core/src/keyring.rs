use crate::{
    error::IvyError,
    io::{read_toml, write_toml, IoError},
    wallet::IvyWallet,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyring {
    /// Path to the keyring toml file
    path: PathBuf,
    ecdsa_keys: HashMap<String, EcdsaKeyFile>,
    bls_keys: HashMap<String, BlsKeyFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlsKeyFile {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcdsaKeyFile {
    pub name: String,
    pub path: PathBuf,
}

impl TryInto<IvyWallet> for EcdsaKeyFile {
    type Error = IvyError;

    /// Attempts to create an IvyWallet from the keyfile. The password is read from the
    /// environment, and returns an error if it is not found.
    fn try_into(self) -> Result<IvyWallet, Self::Error> {
        let pw = std::env::var(&self.name)?;
        IvyWallet::from_keystore(self.path.clone(), &pw)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum KeyringError {
    #[error(transparent)]
    IoError(#[from] IoError),
}

impl Keyring {
    fn new(path: PathBuf) -> Self {
        Keyring { path, ecdsa_keys: HashMap::new(), bls_keys: HashMap::new() }
    }
    fn load(path: PathBuf) -> Result<Self, KeyringError> {
        Ok(read_toml(&path)?)
    }
    fn store(&self) -> Result<(), KeyringError> {
        Ok(write_toml(&self.path, &self)?)
    }
    fn add_ecdsa_keyfile(&mut self, name: String, path: PathBuf) {
        self.ecdsa_keys.insert(name.clone(), EcdsaKeyFile { name, path });
    }
    fn add_bls_keyfile(&mut self, name: String, path: PathBuf) {
        self.bls_keys.insert(name.clone(), BlsKeyFile { name, path });
    }
}

impl Default for Keyring {
    fn default() -> Self {
        let mut keyring_dir = dirs::home_dir().unwrap();
        keyring_dir.push(".ivynet/keys/keyring.toml");
        if !keyring_dir.exists() {
            let parent_path = keyring_dir.parent().expect("Parent path is not reachable");
            std::fs::create_dir_all(parent_path).unwrap();
            let keyring = Keyring::new(keyring_dir);
            keyring.store().expect("Could not store keyring");
            keyring
        } else {
            Keyring::load(keyring_dir).expect("Could not load keyring")
        }
    }
}
