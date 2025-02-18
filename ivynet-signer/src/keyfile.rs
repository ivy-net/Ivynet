use super::IvyWallet;
use ethers::types::Address;
use serde::{Deserialize, Serialize};

use std::{env, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),
    #[error("Environment variable {0} not found for keyname {1}. Ensure the variable is set or in a .env file.")]
    EnvVarError(String, String),
}

impl EcdsaKeyfile {
    pub fn new(path: PathBuf, address: Address, name: &str, pw_env_var: &str) -> Self {
        EcdsaKeyfile { path, address, name: name.to_owned(), pw_env_var: pw_env_var.to_owned() }
    }
    pub fn decrypt(&self, password: String) -> Result<IvyWallet, KeyfileError> {
        IvyWallet::from_keystore(self.path.clone(), &password)
            .map_err(|_| KeyfileError::KeyfileDecryptionError(self.path.display().to_string()))
    }
    pub fn decrypt_env(&self) -> Result<IvyWallet, KeyfileError> {
        // check if the env var is set
        let password = match env::var(&self.pw_env_var) {
            Ok(pw) => pw,
            Err(_) => {
                // Attempt to load from .env if not present
                dotenvy::var(&self.pw_env_var).map_err(|_| {
                    KeyfileError::EnvVarError(self.name.clone(), self.pw_env_var.clone())
                })?
            }
        };
        self.decrypt(password)
    }
}

pub fn prompt_ecdsa_keyfile() -> Result<EcdsaKeyfile, KeyfileError> {
    let path = dialoguer::Input::<String>::new()
        .with_prompt("Enter the full path to the keyfile")
        .interact()?;
    let name =
        dialoguer::Input::<String>::new().with_prompt("Enter the keyfile name.").interact()?;
    let pw_env_var = dialoguer::Input::<String>::new()
        .with_prompt("Enter the password environment variable name. This will be used later to decrypt the keyfile.")
        .interact()?;

    // Key validation
    let pw_value = std::env::var(&pw_env_var)?;
    let wallet = IvyWallet::from_keystore(path.clone().into(), &pw_value)
        .map_err(|_| KeyfileError::KeyfileDecryptionError(path.clone()))?;

    // Derive address for later
    let address = wallet.address();

    Ok(EcdsaKeyfile::new(path.into(), address, &name, &pw_env_var))
}
