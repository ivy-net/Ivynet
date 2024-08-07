use crate::{
    io::{read_toml, write_toml, IoError},
    wallet::{IvyWallet, IvyWalletError},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

pub const DEFAULT_KEY_NAME: &str = "ivy_default";
pub const DEFAULT_KEY_ENV_VAR: &str = "IVY_DEFAULT_KEY_PW";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyring {
    /// Path to the keyring toml file
    pub path: PathBuf,
    pub ecdsa_keys: HashMap<String, EcdsaKeyFile>,
    pub bls_keys: HashMap<String, BlsKeyFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlsKeyFile {
    pub name: String,
    pub path: PathBuf,
    pub env_pw_var: Option<String>,
}

/// Represents a keyfile on disk and an associated name. Name doubles as a reference to an
/// environment variable that holds the password for the keyfile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcdsaKeyFile {
    /// Name of the keyfile. This is also used to reference an environment variable that holds the
    /// password for the keyfile.
    pub name: String,
    pub path: PathBuf,
    pub env_pw_var: Option<String>,
}

impl EcdsaKeyFile {
    pub fn new(name: String, path: PathBuf, env_pw_var: Option<String>) -> Self {
        EcdsaKeyFile { name, path, env_pw_var }
    }

    /// Use the environment variable named by the keyfile's name to get the password for the
    /// keyfile and decrypt. If an environment variable is not found, prompt the user for the
    /// password.
    pub fn try_to_wallet_env_dialog(&self) -> Result<(IvyWallet, String), KeyringError> {
        let pw =  match self.env_pw_var.as_ref() {
            Some(pw) => pw,
            None => {
                &dialoguer::Password::new()
                    .with_prompt("ECDSA password not found in .env, please input password for your default Operator ECDSA keyfile")
                    .interact()?
            }
        };
        Ok((self.try_to_wallet(pw)?, pw.to_owned()))
    }

    /// Use the environment variable named by the keyfile's name to get the password for the
    /// keyfile and decrypt.
    pub fn try_to_wallet_env(&self) -> Result<(IvyWallet, String), KeyringError> {
        let pw = self
            .env_pw_var
            .as_ref()
            .ok_or(KeyringError::EnvVarError(std::env::VarError::NotPresent))?
            .clone();
        Ok((self.try_to_wallet(&pw)?, pw))
    }

    /// Decrypt the keyfile with the provided password.
    pub fn try_to_wallet(&self, password: &str) -> Result<IvyWallet, KeyringError> {
        Ok(IvyWallet::from_keystore(self.path.clone(), password)?)
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
    #[error("Keyfile not found {0}")]
    KeyfileNotFound(String),
    #[error("Keyfile existence check failed, keyfiles not found: {0}")]
    KeyfileCheckFailed(String),
    #[error(transparent)]
    EnvVarError(#[from] std::env::VarError),
    #[error(transparent)]
    WalletError(#[from] IvyWalletError),
    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),
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
    pub fn add_ecdsa_keyfile(&mut self, keyfile: EcdsaKeyFile) -> Result<(), KeyringError> {
        if !keyfile.path.exists() {
            return Err(KeyringError::KeyfileNotFound(keyfile.path.display().to_string()));
        }
        self.ecdsa_keys.insert(keyfile.name.clone(), keyfile);
        Ok(())
    }
    pub fn add_bls_keyfile(&mut self, keyfile: BlsKeyFile) -> Result<(), KeyringError> {
        if !keyfile.path.exists() {
            return Err(KeyringError::KeyfileNotFound(path.display().to_string()));
        }
        self.bls_keys.insert(keyfile.name.clone(), keyfile);
        Ok(())
    }
    pub fn get_ecdsa_keyfile(&self, name: &str) -> Result<&EcdsaKeyFile, KeyringError> {
        self.ecdsa_keys.get(&name).ok_or(KeyringError::KeyfileNotFound(name.to_owned()))
    }
    pub fn get_bls_keyfile(&self, name: &str) -> Result<&BlsKeyFile, KeyringError> {
        self.bls_keys.get(&name).ok_or(KeyringError::KeyfileNotFound(name.to_owned()))
    }
    pub fn default_ecdsa_keyfile(&self) -> Result<&EcdsaKeyFile, KeyringError> {
        self.ecdsa_keys
            .get(&DEFAULT_KEY_NAME)
            .ok_or(KeyringError::KeyfileNotFound(DEFAULT_KEY_NAME.to_owned()))
    }
    pub fn default_bls_keyfile(&self) -> Result<&BlsKeyFile, KeyringError> {
        self.bls_keys
            .get(&DEFAULT_KEY_NAME)
            .ok_or(KeyringError::KeyfileNotFound(DEFAULT_KEY_NAME.to_owned()))
    }
    pub fn remove_ecdsa_keyfile(&mut self, name: &str) -> Result<(), KeyringError> {
        self.ecdsa_keys.remove(&name.to_lowercase());
        Ok(())
    }
    pub fn remove_bls_keyfile(&mut self, name: &str) -> Result<(), KeyringError> {
        self.bls_keys.remove(&name.to_lowercase());
        Ok(())
    }
    pub fn validate_keyfiles_exist(&self) -> Result<(), KeyringError> {
        let nonexistent_keyfiles = self.get_missing_keyfiles();
        if !nonexistent_keyfiles.is_empty() {
            return Err(KeyringError::KeyfileCheckFailed(
                nonexistent_keyfiles
                    .iter()
                    .map(|(_, path)| path.display().to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            ));
        }
        Ok(())
    }
    // pub fn clean_missing_keyfiles(&mut self) -> Result<(), KeyringError> {
    //     let missing_keyfiles = self.get_missing_keyfiles();
    //     for keyfile in missing_keyfiles {
    //         self.ecdsa_keys.remove(&keyfile.display().to_string());
    //         self.bls_keys.remove(&keyfile.display().to_string());
    //     }
    //     Ok(())
    // }
    /// Get a list of keyfiles in the keyring which do not exist on disk
    fn get_missing_keyfiles(&self) -> Vec<(&String, &PathBuf)> {
        let mut missing_keyfiles = vec![];
        for (name, keyfile) in self.ecdsa_keys.iter() {
            if !keyfile.path.exists() {
                missing_keyfiles.push((name, &keyfile.path));
            }
        }
        for (name, keyfile) in self.bls_keys.iter() {
            if !keyfile.path.exists() {
                missing_keyfiles.push((name, &keyfile.path));
            }
        }
        missing_keyfiles
    }
}

#[cfg(test)]
mod keyring_tests {
    use super::*;

    #[test]
    fn test_default_ecdsa_keyfile() {
        let keyring = Keyring::default();
        let default_keyfile = keyring.default_ecdsa_keyfile().unwrap();
    }
}
