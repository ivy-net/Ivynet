use ethers::types::Address;

use crate::wallet::IvyWallet;
use std::{env, path::PathBuf};

pub struct EcdsaKeyfile {
    pub path: PathBuf,
    pub address: Address,
    pub name: String,
    pub pw_env_var: String,
}

#[derive(Debug, thiserror::Error)]
pub enum KeyfileError {
    #[error("No keyfile env var found: {0}")]
    KeyfileEnvVarError(String),
    #[error("Could not decrypt keyfile: {0}")]
    KeyfileDecryptionError(String),
    #[error(transparent)]
    VarError(#[from] std::env::VarError),
}

impl EcdsaKeyfile {
    pub fn new(path: PathBuf, address: Address, name: String, pw_env_var: String) -> Self {
        EcdsaKeyfile { path, address, name, pw_env_var }
    }
    pub fn decrypt(&self, password: String) -> Result<IvyWallet, KeyfileError> {
        IvyWallet::from_keystore(self.path.clone(), &password)
            .map_err(|_| KeyfileError::KeyfileDecryptionError(self.path.display().to_string()))
    }
    pub fn decrypt_env(&self) -> Result<IvyWallet, KeyfileError> {
        let password = env::var(self.pw_env_var.clone())?;
        self.decrypt(password)
    }
}
