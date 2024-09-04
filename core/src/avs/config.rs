use std::{collections::HashMap, path::PathBuf};

use ethers::types::Chain;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

use crate::io::{read_toml, write_toml, IoError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvsConfig {
    // Name access for storage
    pub avs_name: String,
    // Setup map for pathing and is_custom determination
    pub setup_map: HashMap<Chain, Setup>,
    // AVS Specific Settings that can be deserialized
    pub avs_settings: toml::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Setup {
    pub path: PathBuf,
    pub is_custom: bool,
}

impl AvsConfig {
    fn new(avs_name: &str) -> Self {
        let empty_value = toml::Value::from("");
        Self {
            avs_name: String::from(avs_name),
            setup_map: HashMap::new(),
            avs_settings: empty_value,
        }
    }

    pub fn load(avs_name: &str) -> Self {
        let avs_config_path = Self::avs_config_path(avs_name);
        let avs_config: Self = read_toml(&avs_config_path).unwrap_or_else(|_| Self::new(avs_name));
        avs_config
    }

    pub fn store(&self) {
        write_toml(&Self::avs_config_path(&self.avs_name), self)
            .expect("Could not write AVS config");
    }

    // Grabs full path using AVS name from main avs configs directory
    pub fn avs_config_path(avs_name: &str) -> PathBuf {
        dirs::home_dir()
            .expect("Could not get a home directory")
            .join(".ivynet/avs_configs")
            .join(format!("{}.toml", avs_name))
    }

    pub fn get_path(&self, chain: Chain) -> PathBuf {
        if let Some(setup) = self.setup_map.get(&chain) {
            setup.path.clone()
        } else {
            let avs_path: String = dialoguer::Input::new()
                .with_prompt("Input the path for your AVS configuration")
                .interact()
                .expect("Can't decode path");

            // self.setup_map.insert(chain, Setup::new(path.clone(), true));
            PathBuf::from(avs_path)
        }
    }

    pub fn set_path(&mut self, chain: Chain, path: PathBuf, is_custom: bool) {
        self.setup_map.insert(chain, Setup::new(path, is_custom));
    }

    pub fn get_settings(&self) -> toml::Value {
        self.avs_settings.clone()
    }

    pub fn set_settings(&mut self, settings: toml::Value) {
        self.avs_settings = settings;
    }
}

impl Setup {
    pub fn new(path: PathBuf, is_custom: bool) -> Self {
        Self { path, is_custom }
    }
}

#[derive(ThisError, Debug)]
pub enum AvsConfigError {
    #[error(transparent)]
    ConfigIo(#[from] IoError),
}
