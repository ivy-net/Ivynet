use ethers::types::{Chain, H160};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sysinfo::{Disks, System};
use thiserror::Error as ThisError;

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
pub struct IvyConfig {
    /// Storage path for serialized config file
    path: PathBuf,
    /// RPC URL for mainnet
    pub mainnet_rpc_url: String,
    /// RPC URL for holesky
    pub holesky_rpc_url: String,
    // RPC URL for local development
    pub local_rpc_url: String,
    /// Metadata for the operator
    pub metadata: Metadata,
    /// Identification key that node uses for server communications
    pub identity_key: Option<String>,
    /// Default Public Address
    pub default_ether_address: H160,
}

impl Default for IvyConfig {
    fn default() -> Self {
        Self {
            path: DEFAULT_CONFIG_PATH.to_owned(),
            mainnet_rpc_url: "https://rpc.flashbots.net/fast".to_string(),
            holesky_rpc_url: "https://eth-holesky.public.blastapi.io".to_string(),
            local_rpc_url: "http://localhost:8545".to_string(),
            metadata: Metadata::default(),
            identity_key: None,
            default_ether_address: H160::default(),
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

    pub fn set_rpc_url(&mut self, chain: Chain, rpc: &str) -> Result<(), IvyError> {
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

    pub fn set_address(&mut self, address: H160) {
        self.default_ether_address = address;
    }

    pub fn get_rpc_url(&self, chain: Chain) -> Result<String, IvyError> {
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

    pub fn identity_wallet(&self) -> Result<IvyWallet, IvyError> {
        IvyWallet::from_private_key(self.identity_key.clone().ok_or(IvyError::IdentityKeyError)?)
    }

    pub fn uds_dir(&self) -> String {
        format!("{}/ivynet.ipc", self.path.display())
    }
}

pub fn get_system_information() -> Result<(u64, u64, u64), IvyError> {
    let mut sys = System::new();
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list();

    let cpu_cores = sys.cpus().len() as u64;
    let total_memory = sys.total_memory();
    let free_disk = disks[0].available_space();
    Ok((cpu_cores, total_memory, free_disk))
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
