use lazy_static;
use serde_derive::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct IvyConfig {
    // Mainnet rpc url
    pub mainnet_rpc_url: String,
    // Mainnet rpc url
    pub testnet_rpc_url: String,
    // Mainnet rpc url
    pub local_rpc_url: String,
    // Default key file full path
    pub default_keyfile: String,
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
        default_keyfile: "".to_string(),
    };
    store_config(cfg);
}

pub fn set_rpc_url(rpc: String, network: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = CONFIG.lock()?;
    match network.as_str() {
        "mainnet" => {
            println!("Setting mainnet rpc url to: {}", rpc);
            cfg.mainnet_rpc_url = rpc;
        }
        "testnet" => {
            println!("Setting testnet rpc url to: {}", rpc);
            cfg.testnet_rpc_url = rpc;
        }
        "local" => {
            println!("Setting local rpc url to: {}", rpc);
            cfg.local_rpc_url = rpc;
        }
        _ => {
            println!("Unknown network");
        }
    }
    store_config(cfg.clone());
    Ok(())
}

pub fn get_rpc_url(network: String) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = CONFIG.lock()?;
    match network.as_str() {
        "mainnet" => println!("Mainnet url: {:?}", cfg.mainnet_rpc_url),
        "testnet" => println!("Testnet url: {:?}", cfg.testnet_rpc_url),
        "local" => println!("Localhost url: {:?}", cfg.local_rpc_url),
        _ => {
            println!("Unknown network: {}", network);
        }
    }
    Ok(())
}

pub fn set_default_keyfile(keyfile: String) {
    let mut cfg = CONFIG.lock().unwrap();
    cfg.default_keyfile = keyfile;
    store_config(cfg.clone());
}
