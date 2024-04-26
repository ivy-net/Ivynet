use lazy_static;
use serde_derive::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};
use sys_info::{self, DiskInfo, MemInfo};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct IvyConfig {
    // Mainnet rpc url
    pub mainnet_rpc_url: String,
    // Mainnet rpc url
    pub testnet_rpc_url: String,
    // Mainnet rpc url
    pub local_rpc_url: String,
    // Default private key file full path
    pub default_private_keyfile: PathBuf,
    // Default public key file full path
    pub default_public_keyfile: PathBuf,
}

lazy_static::lazy_static! {
    static ref CONFIG: Mutex<IvyConfig> = Mutex::new(load_config());
}

pub fn load_config() -> IvyConfig {
    match confy::load::<IvyConfig>("ivy", "ivy-config") {
        Ok(cfg) => cfg,
        Err(_) => {
            // The configuration file likely does not exist, so create a new one
            create_new_config();
            // Then try loading the configuration again
            confy::load::<IvyConfig>("ivy", "ivy-config").expect("Failed to load config")
        }
    }
}

pub fn store_config(cfg: IvyConfig) {
    confy::store("ivy", "ivy-config", &cfg).expect("Failed to store config");
}

pub fn get_config() -> IvyConfig {
    CONFIG.lock().unwrap().clone()
}

fn create_new_config() {
    let cfg = IvyConfig {
        mainnet_rpc_url: "https://rpc.flashbots.net/fast".to_string(),
        testnet_rpc_url: "https://rpc.holesky.ethpandaops.io".to_string(),
        local_rpc_url: "http://localhost:8545".to_string(),
        default_private_keyfile: "".into(),
        default_public_keyfile: "".into(),
    };
    store_config(cfg);
}

pub fn set_rpc_url(rpc: &str, network: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = CONFIG.lock()?;
    match network {
        "mainnet" => {
            println!("Setting mainnet rpc url to: {}", rpc);
            cfg.mainnet_rpc_url = rpc.to_string();
        }
        "testnet" => {
            println!("Setting testnet rpc url to: {}", rpc);
            cfg.testnet_rpc_url = rpc.to_string();
        }
        "local" => {
            println!("Setting local rpc url to: {}", rpc);
            cfg.local_rpc_url = rpc.to_string();
        }
        _ => {
            println!("Unknown network");
        }
    }
    store_config(cfg.clone());
    Ok(())
}

pub fn get_rpc_url(network: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = CONFIG.lock()?;
    match network {
        "mainnet" => println!("Mainnet url: {:?}", cfg.mainnet_rpc_url),
        "testnet" => println!("Testnet url: {:?}", cfg.testnet_rpc_url),
        "local" => println!("Localhost url: {:?}", cfg.local_rpc_url),
        _ => {
            println!("Unknown network: {}", network);
        }
    }
    Ok(())
}

pub fn set_default_private_keyfile(keyfile: PathBuf) {
    let mut cfg = CONFIG.lock().unwrap();
    cfg.default_private_keyfile = keyfile;
    store_config(cfg.clone());
}

pub fn get_default_private_keyfile() -> PathBuf {
    let cfg = CONFIG.lock().unwrap();
    cfg.default_private_keyfile.clone()
}

pub fn set_default_public_keyfile(keyfile: PathBuf) {
    let mut cfg = CONFIG.lock().unwrap();
    cfg.default_public_keyfile = keyfile;
    store_config(cfg.clone());
}

pub fn get_default_public_keyfile() -> PathBuf {
    let cfg = CONFIG.lock().unwrap();
    cfg.default_public_keyfile.clone().into()
    
}

pub fn get_system_information() -> Result<(u32, MemInfo, DiskInfo), Box<dyn std::error::Error>> {
    let cpu_cores = sys_info::cpu_num()?;
    let mem_info = sys_info::mem_info()?;
    let disk_info = sys_info::disk_info()?;
    Ok((cpu_cores, mem_info, disk_info))
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
       println!("Config: {:?}", load_config());
    }
}