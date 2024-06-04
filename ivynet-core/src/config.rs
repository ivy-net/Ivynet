use ethers::types::Chain;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sysinfo::{Disks, System};

use crate::{error::IvyError, metadata::Metadata};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IvyConfig {
    pub mainnet_rpc_url: String,
    pub holesky_rpc_url: String,
    pub local_rpc_url: String,
    /// Default private key file full path
    pub default_private_keyfile: PathBuf,
    /// Default public key file full path
    pub default_public_keyfile: PathBuf,
    pub metadata: Option<Metadata>,
}

impl Default for IvyConfig {
    fn default() -> Self {
        Self {
            mainnet_rpc_url: "https://rpc.flashbots.net/fast".to_string(),
            holesky_rpc_url: "https://rpc.holesky.ethpandaops.io".to_string(),
            local_rpc_url: "http://localhost:8545".to_string(),
            default_private_keyfile: "".into(), // TODO: Option
            default_public_keyfile: "".into(),
            metadata: None,
        }
    }
}

impl IvyConfig {
    pub fn new() -> Self {
        Self {
            mainnet_rpc_url: "https://rpc.flashbots.net/fast".to_string(),
            holesky_rpc_url: "https://rpc.holesky.ethpandaops.io".to_string(),
            local_rpc_url: "http://localhost:8545".to_string(),
            default_private_keyfile: "".into(),
            default_public_keyfile: "".into(),
            ..Default::default()
        }
    }

    // TODO: Consider making this fallible / requiring an init.
    pub fn load() -> Self {
        match confy::load::<IvyConfig>("ivy", "ivy-config") {
            Ok(cfg) => cfg,
            Err(_) => Self::new(),
        }
    }

    pub fn store(&self) -> Result<(), IvyError> {
        confy::store("ivy", "ivy-config", self)?;
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

    pub fn get_path(&self) -> Result<PathBuf, IvyError> {
        Ok(confy::get_configuration_file_path("ivy", "ivy-config")?)
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
    use super::*;

    #[test]
    fn test_load_config() {
        println!("Config: {:?}", IvyConfig::load());
    }
}
