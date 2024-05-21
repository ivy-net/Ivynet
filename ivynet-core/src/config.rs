use once_cell::sync::Lazy;
use serde_derive::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::sync::Mutex;
use sysinfo::{Disks, System};

use crate::rpc_management::Network;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct IvyConfig {
    // Mainnet rpc url
    pub mainnet_rpc_url: String,
    // Mainnet rpc url
    pub holesky_rpc_url: String,
    // Mainnet rpc url
    pub local_rpc_url: String,
    // Default private key file full path
    pub default_private_keyfile: PathBuf,
    // Default public key file full path
    pub default_public_keyfile: PathBuf,
}

// NOTE: This structure may still encounter race conditions due to execution flow.
pub static CONFIG: Lazy<Mutex<IvyConfig>> = Lazy::new(|| IvyConfig::load().into());

impl IvyConfig {
    pub fn new() -> Self {
        Self {
            mainnet_rpc_url: "https://rpc.flashbots.net/fast".to_string(),
            holesky_rpc_url: "https://rpc.holesky.ethpandaops.io".to_string(),
            local_rpc_url: "http://localhost:8545".to_string(),
            default_private_keyfile: "".into(),
            default_public_keyfile: "".into(),
        }
    }

    // TODO: Consider making this fallible / requiring an init.
    pub fn load() -> Self {
        match confy::load::<IvyConfig>("ivy", "ivy-config") {
            Ok(cfg) => cfg,
            Err(_) => Self::new(),
        }
    }

    pub fn store(&self) -> Result<(), Box<dyn Error>> {
        confy::store("ivy", "ivy-config", self)?;
        Ok(())
    }

    pub fn set_rpc_url(&mut self, network: Network, rpc: &str) -> Result<(), Box<dyn Error>> {
        match network {
            Network::Mainnet => {
                println!("Setting mainnet rpc url to: {}", rpc);
                self.mainnet_rpc_url = rpc.to_string();
            }
            Network::Holesky => {
                println!("Setting holesky rpc url to: {}", rpc);
                self.holesky_rpc_url = rpc.to_string();
            }
            Network::Local => {
                println!("Setting local rpc url to: {}", rpc);
                self.local_rpc_url = rpc.to_string();
            }
            _ => return Err("Unknown network".into()),
        }
        Ok(())
    }

    pub fn set_public_keyfile(&mut self, keyfile: PathBuf) {
        self.default_public_keyfile = keyfile;
    }

    pub fn set_private_keyfile(&mut self, keyfile: PathBuf) {
        self.default_private_keyfile = keyfile;
    }

    pub fn get_rpc_url(&self, network: Network) -> Result<String, Box<dyn std::error::Error>> {
        Ok(match network {
            Network::Mainnet => self.mainnet_rpc_url.clone(),
            Network::Holesky => self.holesky_rpc_url.clone(),
            Network::Local => self.local_rpc_url.clone(),
        })
    }

    pub fn get_path(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        Ok(confy::get_configuration_file_path("ivy", "ivy-config")?)
    }
}

// pub fn load_config() -> IvyConfig {
//     match confy::load::<IvyConfig>("ivy", "ivy-config") {
//         Ok(cfg) => cfg,
//         Err(_) => {
//             // The configuration file likely does not exist, so create a new one
//             create_new_config();
//             // Then try loading the configuration again
//             confy::load::<IvyConfig>("ivy", "ivy-config").expect("Failed to load config")
//         }
//     }
// }
//
// pub fn store_config(cfg: IvyConfig) {
//     confy::store("ivy", "ivy-config", cfg).expect("Failed to store config");
// }
//
// pub fn get_config() -> IvyConfig {
//     CONFIG.clone()
// }
//
// fn create_new_config() {
//     let cfg = IvyConfig {
//         mainnet_rpc_url: "https://rpc.flashbots.net/fast".to_string(),
//         holesky_rpc_url: "https://rpc.holesky.ethpandaops.io".to_string(),
//         local_rpc_url: "http://localhost:8545".to_string(),
//         default_private_keyfile: "".into(),
//         default_public_keyfile: "".into(),
//     };
//     store_config(cfg);
// }
//
// pub fn set_rpc_url(rpc: &str, network: &str) -> Result<(), Box<dyn std::error::Error>> {
//     println!("{}", network);
//     let mut config = CONFIG.clone();
//     match network {
//         "mainnet" => {
//             println!("Setting mainnet rpc url to: {}", rpc);
//             config.mainnet_rpc_url = rpc.to_string();
//         }
//         "holesky" => {
//             println!("Setting holesky rpc url to: {}", rpc);
//             config.holesky_rpc_url = rpc.to_string();
//         }
//         "local" => {
//             println!("Setting local rpc url to: {}", rpc);
//             config.local_rpc_url = rpc.to_string();
//         }
//         _ => {
//             println!("Unknown network");
//         }
//     }
//     store_config(config);
//     Ok(())
// }
//
// pub fn get_rpc_url(network: Network) -> Result<String, Box<dyn std::error::Error>> {
//     Ok(match network {
//         Network::Mainnet => CONFIG.mainnet_rpc_url.clone(),
//         Network::Holesky => CONFIG.holesky_rpc_url.clone(),
//         Network::Local => CONFIG.local_rpc_url.clone(),
//     })
// }
//
// pub fn set_default_private_keyfile(keyfile: PathBuf) {
//     let mut config = CONFIG.clone();
//     config.default_private_keyfile = keyfile;
//     println!("SAVE_CONFIG: {:#?}", config);
//     store_config(config);
// }
//
// pub fn get_default_private_keyfile() -> PathBuf {
//     CONFIG.default_private_keyfile.clone()
// }
//
// pub fn set_default_public_keyfile(keyfile: PathBuf) {
//     let mut config = CONFIG.clone();
//     config.default_public_keyfile = keyfile;
//     store_config(config);
// }

pub fn get_system_information() -> Result<(u64, u64, u64), Box<dyn Error>> {
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
