use serde_derive::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct IvyConfig {
    // Default rpc url
    pub rpc_url: String,
    // Default key file full path
    pub default_keyfile: String,
}

pub fn load_config() -> IvyConfig {
    confy::load("ivy", "ivy-config").expect("Failed to load config")
}

pub fn store_config(cfg: IvyConfig) {
    println!("Config in store config: {:?}", cfg);
    confy::store("ivy", "ivy-config", &cfg).expect("Failed to store config");
}

pub fn set_rpc_url(rpc: String) {
    let mut cfg = load_config();
    println!("Setting rpc url to: {}", rpc);
    cfg.rpc_url = rpc;
    
    store_config(cfg);
}

pub fn set_default_keyfile(keyfile: String) {
    let mut cfg = load_config();
    cfg.default_keyfile = keyfile;
    store_config(cfg);
}




#[test]
fn test_set_rpc() {
    set_rpc_url("http://notreallyanrpchost:8845".to_string());
    let cfg = load_config();
    assert_eq!(cfg.rpc_url, "http://notreallyanrpchost:8845");
}