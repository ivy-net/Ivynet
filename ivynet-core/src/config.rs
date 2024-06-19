use ethers::types::Chain;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sysinfo::{Disks, System};

pub static DEFAULT_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = dirs::home_dir().expect("Could not get a home directory");
    path.join(".ivynet")
});

use crate::{
    error::IvyError,
    metadata::Metadata,
    utils::{read_toml, write_toml},
    wallet::IvyWallet,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IvyConfig {
    /// Storage path for serialized config file
    path: PathBuf,
    pub mainnet_rpc_url: String,
    pub holesky_rpc_url: String,
    pub local_rpc_url: String,
    /// Default private key file full path
    pub default_private_keyfile: PathBuf,
    /// Default public key file full path
    pub default_public_keyfile: PathBuf,
    pub metadata: Metadata,
    // Identification key that node uses for server communications
    pub identity_key: Option<String>,
}

impl Default for IvyConfig {
    fn default() -> Self {
        Self {
            path: DEFAULT_CONFIG_PATH.to_owned(),
            mainnet_rpc_url: "https://rpc.flashbots.net/fast".to_string(),
            holesky_rpc_url: "https://rpc.holesky.ethpandaops.io".to_string(),
            local_rpc_url: "http://localhost:8545".to_string(),
            default_private_keyfile: "".into(), // TODO: Option
            default_public_keyfile: "".into(),
            metadata: Metadata::default(),
            identity_key: None,
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

    pub fn load(path: PathBuf) -> Result<Self, IvyError> {
        let config: Self = read_toml(path)?;
        Ok(config)
    }

    pub fn load_from_default_path() -> Result<Self, IvyError> {
        let config_path = DEFAULT_CONFIG_PATH.to_owned().join("ivy-config.toml");
        Self::load(config_path)
    }

    pub fn store(&self) -> Result<(), IvyError> {
        let config_path = self.path.clone().join("ivy-config.toml");
        write_toml(config_path, self)?;
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

    pub fn set_public_keyfile(&mut self, keyfile: PathBuf) {
        self.default_public_keyfile = keyfile;
    }

    pub fn set_private_keyfile(&mut self, keyfile: PathBuf) {
        self.default_private_keyfile = keyfile;
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_load_config() {
        todo!();
    }
}
