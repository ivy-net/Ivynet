use crate::{
    error::IvyError,
    io::{read_toml, write_toml, IoError},
    wallet::IvyWallet,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

pub const DEFAULT_KEY_ID: &str = "DEFAULT_ECDSA_KEYFILE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyring {
    /// Path to the keyring toml file
    pub path: PathBuf,
    ecdsa_keys: HashMap<String, EcdsaKeyFile>,
    bls_keys: HashMap<String, BlsKeyFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlsKeyFile {
    pub name: String,
    pub path: PathBuf,
}

/// Represents a keyfile on disk and an associated name. Name doubles as a reference to an
/// environment variable that holds the password for the keyfile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcdsaKeyFile {
    pub name: String,
    pub path: PathBuf,
}

impl EcdsaKeyFile {
    pub fn new(name: String, path: PathBuf) -> Self {
        EcdsaKeyFile { name, path }
    }

    pub fn try_to_wallet_with_env_password(&self) -> Result<IvyWallet, IvyError> {
        let pw = std::env::var(&self.name)?;
        self.try_to_wallet(&pw)
    }

    pub fn try_to_wallet(&self, password: &str) -> Result<IvyWallet, IvyError> {
        IvyWallet::from_keystore(self.path.clone(), password)
    }
}

impl Default for Keyring {
    fn default() -> Self {
        Keyring { path: Self::default_path(), ecdsa_keys: HashMap::new(), bls_keys: HashMap::new() }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum KeyringError {
    #[error(transparent)]
    IoError(#[from] IoError),
    #[error("Keyring not found at path {0}")]
    PathError(String),
    #[error("Keyfile not found")]
    KeyfileNotFound(String),
}

impl Keyring {
    pub fn new(path: PathBuf) -> Self {
        Keyring { path, ecdsa_keys: HashMap::new(), bls_keys: HashMap::new() }
    }
    pub fn load(path: PathBuf) -> Result<Self, KeyringError> {
        Ok(read_toml(&path)?)
    }
    pub fn load_default() -> Result<Self, KeyringError> {
        Self::load(Self::default_path())
    }
    pub fn store(&self) -> Result<(), KeyringError> {
        Ok(write_toml(&self.path, &self)?)
    }
    pub fn add_ecdsa_keyfile(&mut self, name: &str, path: PathBuf) {
        self.ecdsa_keys.insert(name.to_lowercase(), EcdsaKeyFile { name: name.to_owned(), path });
    }
    pub fn add_bls_keyfile(&mut self, name: &str, path: PathBuf) {
        self.bls_keys.insert(name.to_lowercase(), BlsKeyFile { name: name.to_owned(), path });
    }
    pub fn get_ecdsa_keyfile(&self, name: &str) -> Option<&EcdsaKeyFile> {
        self.ecdsa_keys.get(&name.to_lowercase())
    }
    pub fn get_bls_keyfile(&self, name: &str) -> Option<&BlsKeyFile> {
        self.bls_keys.get(&name.to_lowercase())
    }
    pub fn default_ecdsa_keyfile(&self) -> Option<&EcdsaKeyFile> {
        self.ecdsa_keys.get(DEFAULT_KEY_ID)
    }
    pub fn default_bls_keyfile(&self) -> Option<&BlsKeyFile> {
        self.bls_keys.get(DEFAULT_KEY_ID)
    }
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .expect("Could not get a home directory")
            .join(".ivynet")
            .join("ivy-keyring.toml")
    }
    pub fn keyring_dir(&self) -> Result<PathBuf, KeyringError> {
        let path =
            self.path.parent().ok_or(KeyringError::PathError(self.path.display().to_string()))?;
        Ok(PathBuf::from(path))
    }
}
