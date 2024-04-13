use lazy_static;
use serde_derive::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct IvyConfig {
    // Default rpc url
    pub rpc_url: String,
    // Default key file full path
    pub default_keyfile: String,
}

lazy_static::lazy_static! {
    static ref CONFIG: Mutex<IvyConfig> = Mutex::new(load_config());
}

pub fn load_config() -> IvyConfig {
    confy::load("ivy", "ivy-config").expect("Failed to load config")
}

pub fn store_config(cfg: IvyConfig) {
    confy::store("ivy", "ivy-config", &cfg).expect("Failed to store config");
}

pub fn get_config() -> IvyConfig {
    CONFIG.lock().unwrap().clone()
}

pub fn set_rpc_url(rpc: String) {
    let mut cfg = CONFIG.lock().unwrap();
    println!("Setting rpc url to: {}", rpc);
    cfg.rpc_url = rpc;
    store_config(cfg.clone());
}

pub fn set_default_keyfile(keyfile: String) {
    let mut cfg = CONFIG.lock().unwrap();
    cfg.default_keyfile = keyfile;
    store_config(cfg.clone());
}
