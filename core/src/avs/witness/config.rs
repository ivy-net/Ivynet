use crate::{
    io::{read_toml, write_toml, IoError},
    keys::keyfile::EcdsaKeyfile,
};
use ethers::types::Chain;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tracing::info;

/// Mapping of chains to their respective witness configurations.
pub type WitnessChainConfigs = HashMap<Chain, WitnessConfig>;

/// Config for witnesschain AVS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessConfig {
    /// Self-reference to file storage path
    pub path: PathBuf,
    pub operator_ecdsa_file: Option<EcdsaKeyfile>,
    pub watchtower_ecdsa_file: Option<EcdsaKeyfile>,
}

#[derive(Debug, thiserror::Error)]
pub enum WitnessConfigError {
    #[error(transparent)]
    IoError(#[from] IoError),
    #[error(transparent)]
    FileIoError(#[from] std::io::Error),
}

// TODO: The witness CLI allows for multiple watchtower keys. We may want to add this feature here.
/// Config for witnesschain AVS. Contains path to the witness config file, operator and watchtower
/// ECDSA keys.
impl WitnessConfig {
    pub fn new(
        path: PathBuf,
        operator_ecdsa_file: Option<EcdsaKeyfile>,
        watchtower_ecdsa_file: Option<EcdsaKeyfile>,
    ) -> Self {
        WitnessConfig { path, operator_ecdsa_file, watchtower_ecdsa_file }
    }
    pub fn load(path: PathBuf) -> Result<Self, WitnessConfigError> {
        Ok(read_toml(&path)?)
    }
    pub fn store(&self) -> Result<(), WitnessConfigError> {
        if !self.path.exists() {
            let parent_path = self.path.parent().expect("Parent path is not reachable");
            std::fs::create_dir_all(parent_path)?;
        }
        info!("Storing witness config at {:?}", &self.path);
        Ok(write_toml(&self.path, &self)?)
    }
    pub fn load_from_default_path() -> Result<Self, WitnessConfigError> {
        let path = Self::default_path();
        if !path.exists() {
            let parent_path = path.parent().expect("Parent path is not reachable");
            std::fs::create_dir_all(parent_path).unwrap();
            let config = WitnessConfig::new(path, None, None);
            config.store().expect("Could not store config");
            Ok(config)
        } else {
            WitnessConfig::load(path)
        }
    }
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .expect("Could not find config directory")
            .join(".eigenlayer/witness/witness_config.toml")
    }
}

impl Default for WitnessConfig {
    fn default() -> Self {
        WitnessConfig {
            path: dirs::home_dir()
                .expect("Could not find config directory")
                .join(".eigenlayer/witness/witness_config.toml"),
            operator_ecdsa_file: None,
            watchtower_ecdsa_file: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct WitnessRunConfig {
    private_key: String,
    watchtower_address: String,
}

#[cfg(test)]
mod test_witness_config {
    use super::*;
    use crate::wallet::IvyWallet;
    use tempfile::tempdir;

    #[test]
    fn test_store_witness_config() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("witness_config.toml");

        let operator_wallet = IvyWallet::new();
        let operator_address = operator_wallet.address();
        let operator_password = "operator";
        let operator_name = "operator";
        let operator_pw_env_var = "OPERATOR_PW";
        let operator_keyfile_path = operator_wallet
            .encrypt_and_store(temp_dir.path(), operator_name, operator_password)
            .unwrap();
        let operator_ecdsa_file = EcdsaKeyfile::new(
            operator_keyfile_path,
            operator_address,
            operator_name,
            operator_pw_env_var,
        );

        let watchtower_wallet = IvyWallet::new();
        let watchtower_address = watchtower_wallet.address();
        let watchtower_password = "watchtower";
        let watchtower_name = "watchtower";
        let watchtower_pw_env_var = "WATCHTOWER_PW";
        let watchtower_keyfile_path = watchtower_wallet
            .encrypt_and_store(temp_dir.path(), watchtower_name, watchtower_password)
            .unwrap();
        let watchtower_ecdsa_file = EcdsaKeyfile::new(
            watchtower_keyfile_path,
            watchtower_address,
            watchtower_name,
            watchtower_pw_env_var,
        );

        let witness_config = WitnessConfig::new(
            path.clone(),
            Some(operator_ecdsa_file),
            Some(watchtower_ecdsa_file),
        );
        witness_config.store().unwrap();
        let witness_config_loaded = WitnessConfig::load(path).unwrap();
        assert_eq!(witness_config, witness_config_loaded);
    }
    #[test]
    fn test_load_witness_conifg_from_default_path() {
        let new_config = WitnessConfig::new(
            dirs::home_dir().unwrap().join(".eigenlayer/witness/witness_config.toml"),
            None,
            None,
        );
        let witness_config_loaded = WitnessConfig::load_from_default_path().unwrap();
        assert_eq!(new_config, witness_config_loaded);
    }
}
