use ethers::types::Chain;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error as ThisError;
use tonic::transport::Uri;
use uuid::Uuid;

pub static DEFAULT_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = dirs::home_dir().expect("Could not get a home directory");
    path.join(".ivynet")
});

use crate::{
    error::IvyError,
    io::{read_toml, write_toml, IoError},
    metadata::Metadata,
    wallet::{IvyWallet, IvyWalletError},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendInfo {
    pub server_url: String,
    pub server_ca: String,
    /// Identification key that node uses for server communications
    pub identity_key: String,
}

// TODO: Change rpc urls to hashmap or remove entirely
// add reference to keyfile for identity keys instead of using provider id
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IvyConfig {
    /// Storage path for serialized config file
    path: PathBuf,
    /// Machines Id
    pub machine_id: Uuid,
    /// RPC URL for mainnet
    pub mainnet_rpc_url: String,
    /// RPC URL for holesky
    pub holesky_rpc_url: String,
    // RPC URL for local development
    pub local_rpc_url: String,
    /// Metadata for the operator
    pub metadata: Metadata,
    /// Web server information
    pub backend_info: BackendInfo,
}

impl Default for IvyConfig {
    fn default() -> Self {
        Self {
            path: DEFAULT_CONFIG_PATH.to_owned(),
            machine_id: Uuid::new_v4(),
            mainnet_rpc_url: "https://rpc.flashbots.net/fast".to_string(),
            holesky_rpc_url: "https://eth-holesky.public.blastapi.io".to_string(),
            local_rpc_url: "http://localhost:8545".to_string(),
            metadata: Metadata::default(),
            backend_info: BackendInfo {
                server_url: "https://api1.test.ivynet.dev".into(),
                server_ca: "".into(),
                identity_key: "".into(),
            },
        }
    }
}

impl IvyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_at_path(path: PathBuf) -> Self {
        Self { path, ..Default::default() }
    }

    pub fn load(path: PathBuf) -> Result<Self, ConfigError> {
        let config: Self = read_toml(&path)?;
        Ok(config)
    }

    pub fn load_from_default_path() -> Result<Self, ConfigError> {
        let config_path = DEFAULT_CONFIG_PATH.to_owned().join("ivy-config.toml");
        //Previous impl built a bad path - let this error properly
        Self::load(config_path)
    }

    pub fn store(&self) -> Result<(), ConfigError> {
        // TODO: Assert identity key is None on save
        let config_path = self.path.clone().join("ivy-config.toml");
        write_toml(&config_path, self)?;
        Ok(())
    }

    pub fn set_default_rpc_url(&mut self, chain: Chain, rpc: &str) -> Result<(), IvyError> {
        match chain {
            Chain::Mainnet => {
                println!("Setting mainnet rpc url to: {}", rpc);
                self.mainnet_rpc_url = rpc.to_string();
            }
            Chain::Holesky => {
                println!("Setting holesky rpc url to: {}", rpc);
                self.holesky_rpc_url = rpc.to_string();
            }
            Chain::AnvilHardhat => {
                println!("Setting local rpc url to: {}", rpc);
                self.local_rpc_url = rpc.to_string();
            }
            _ => return Err(IvyError::UnknownNetwork),
        }
        Ok(())
    }

    pub fn get_default_rpc_url(&self, chain: Chain) -> Result<String, IvyError> {
        match chain {
            Chain::Mainnet => Ok(self.mainnet_rpc_url.clone()),
            Chain::Holesky => Ok(self.holesky_rpc_url.clone()),
            Chain::AnvilHardhat => Ok(self.local_rpc_url.clone()),
            _ => Err(IvyError::Unimplemented),
        }
    }

    pub fn get_path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Get the path to the directory containing the ivy-config.toml file.
    pub fn get_dir(&self) -> PathBuf {
        self.path.clone()
    }

    /// Get the path to the ivy-config.toml file.
    pub fn get_file(&self) -> PathBuf {
        self.path.join("ivy-config.toml")
    }

    pub fn identity_wallet(&self) -> Result<IvyWallet, IvyError> {
        Ok(IvyWallet::from_private_key(self.backend_info.identity_key.clone())?)
    }

    pub fn set_server_url(&mut self, url: String) {
        self.backend_info.server_url = url;
    }

    pub fn get_server_url(&self) -> Result<Uri, IvyError> {
        Uri::try_from(self.backend_info.server_url.clone()).map_err(|_| IvyError::InvalidUri)
    }

    pub fn set_server_ca(&mut self, ca: String) {
        self.backend_info.server_ca = ca;
    }

    pub fn get_server_ca(&self) -> String {
        self.backend_info.server_ca.clone()
    }

    pub fn uds_dir(&self) -> String {
        format!("{}/ivynet.ipc", self.path.display())
    }
}

#[derive(ThisError, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    ConfigIo(#[from] IoError),
    #[error(transparent)]
    WalletFetchError(#[from] IvyWalletError),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load_config_error() {
        let path = PathBuf::from("nonexistent");
        let config = IvyConfig::load(path);
        println!("{:?}", config);
        assert!(config.is_err());
    }

    #[test]
    fn test_uds_dir() {
        let config = super::IvyConfig::default();
        let path_str = config.path.display().to_string();
        let uds_dir = config.uds_dir();
        assert_eq!(uds_dir, path_str + "/ivynet.ipc");
        println!("{}", uds_dir);
    }
}
