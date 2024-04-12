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
    confy::store("ivy", "ivy-config", &cfg).expect("Failed to store config");
}

pub fn set_rpc_url(rpc_url: String) {
    let mut cfg = load_config();
    cfg.rpc_url = rpc_url;
    store_config(cfg);
}

pub fn set_default_keyfile(keyfile: String) {
    let mut cfg = load_config();
    cfg.default_keyfile = keyfile;
    store_config(cfg);
}

