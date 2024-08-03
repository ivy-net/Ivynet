use crate::io::{read_toml, write_toml, IoError};
use ethers::types::Chain;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

/// Mapping of chains to their respective witness configurations.
pub type WitnessChainConfigs = HashMap<Chain, WitnessConfig>;

/// Config for witnesschain AVS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessConfig {
    /// Self-reference to file storage path
    pub path: PathBuf,
    pub operator_ecdsa_key_path: PathBuf,
    pub watchtower_ecdsa_key_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum WitnessConfigError {
    #[error(transparent)]
    IoError(#[from] IoError),
}

impl WitnessConfig {
    pub fn new(
        path: PathBuf,
        operator_ecdsa_key_path: impl Into<PathBuf>,
        watchtower_ecdsa_key_path: impl Into<PathBuf>,
    ) -> Self {
        WitnessConfig {
            path,
            operator_ecdsa_key_path: operator_ecdsa_key_path.into(),
            watchtower_ecdsa_key_path: watchtower_ecdsa_key_path.into(),
        }
    }

    pub fn load(path: PathBuf) -> Result<Self, WitnessConfigError> {
        Ok(read_toml(&path)?)
    }

    pub fn store(&self) -> Result<(), WitnessConfigError> {
        Ok(write_toml(&self.path, &self)?)
    }
}

#[cfg(test)]
mod test_witness_config {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_store_witness_config() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("witness_config.toml");
        let witness_config = WitnessConfig::new(
            path.clone(),
            temp_dir.path().join("operator_ecdsa_key"),
            temp_dir.path().join("watchtower_ecdsa_key"),
        );
        witness_config.store().unwrap();
        let witness_config_loaded = WitnessConfig::load(path).unwrap();
        assert_eq!(witness_config, witness_config_loaded);
    }
}
