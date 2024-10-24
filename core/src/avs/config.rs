use std::{collections::HashMap, fs::create_dir_all, path::PathBuf};

use ethers::types::{Chain, H160};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
use tracing::info;

use crate::io::{read_toml, write_toml, IoError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvsConfig {
    // Name access for storage
    pub avs_name: String,
    // Setup map for pathing and is_custom determination
    pub setup_map: HashMap<Chain, Setup>,
    // AVS Specific Settings that can be deserialized
    pub avs_settings: HashMap<Chain, toml::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Setup {
    pub path: PathBuf,
    pub operator_address: H160,
    pub is_custom: bool,
}

impl AvsConfig {
    pub fn new(avs_name: &str) -> Self {
        Self {
            avs_name: String::from(avs_name),
            setup_map: HashMap::new(),
            //Abstract and empty on purpose - avs should use and modify as needed
            avs_settings: HashMap::new(),
        }
    }

    pub fn load(avs_name: &str) -> Result<Self, AvsConfigError> {
        if !AvsConfig::exists(avs_name) {
            let configs_path = dirs::home_dir()
                .expect("Could not get a home directory")
                .join(".ivynet/avs_configs");
            create_dir_all(configs_path).expect("Could not create AVS configs directory");
            AvsConfig::new(avs_name).store();
        }
        let avs_config_path = Self::avs_config_path(avs_name);
        let avs_config: Self = read_toml(&avs_config_path)?;
        Ok(avs_config)
    }

    pub fn exists(name: &str) -> bool {
        let toml_path = dirs::home_dir()
            .expect("Could not get a home directory")
            .join(".ivynet/avs_configs")
            .join(format!("{}.toml", name));
        info!("{}", toml_path.exists());
        toml_path.exists()
    }

    pub fn ask_for_path() -> PathBuf {
        let path = dialoguer::Input::<String>::new()
            .with_prompt(
                "Enter the path of the directory containining the AVS's docker-compose.yml",
            )
            .interact()
            .expect("Could not get path");

        Self::strip_docker_compose(PathBuf::from(path))

        //TODO: Validate docker-compose.yml exists within directory
    }

    fn strip_docker_compose(path: PathBuf) -> PathBuf {
        let mut path = path;
        if path.ends_with("docker-compose.yml") {
            path.pop(); // Remove "docker-compose.yml"
        }
        path
    }

    pub fn store(&self) {
        let path = &Self::avs_config_path(&self.avs_name);
        write_toml(path, self).expect("Could not write AVS config");
    }

    // Grabs full path using AVS name from main avs configs directory
    pub fn avs_config_path(avs_name: &str) -> PathBuf {
        dirs::home_dir()
            .expect("Could not get a home directory")
            .join(".ivynet/avs_configs")
            .join(format!("{}.toml", avs_name))
    }

    pub fn log_path(avs_name: &str, chain: &str) -> PathBuf {
        dirs::home_dir()
            .expect("Could not get a home directory")
            .join(".ivynet/logs")
            .join(avs_name)
            .join(chain)
    }

    pub fn get_path(&self, chain: Chain) -> PathBuf {
        self.setup_map
            .get(&chain)
            .expect("No path found - please run the setup command")
            .path
            .clone()
    }

    pub fn set_path(
        &mut self,
        chain: Chain,
        path: PathBuf,
        operator_address: H160,
        is_custom: bool,
    ) {
        self.setup_map.insert(chain, Setup::new(path, operator_address, is_custom));
    }

    pub fn get_settings(&self, chain: Chain) -> toml::Value {
        self.avs_settings
            .get(&chain)
            .unwrap_or_else(|| panic!("No settings found for {}", chain))
            .clone()
    }

    pub fn set_settings(&mut self, chain: Chain, settings: toml::Value) {
        self.avs_settings.insert(chain, settings);
    }
}

impl Setup {
    pub fn new(path: PathBuf, operator_address: H160, is_custom: bool) -> Self {
        Self { path, operator_address, is_custom }
    }
}

#[derive(ThisError, Debug)]
pub enum AvsConfigError {
    #[error(transparent)]
    ConfigIo(#[from] IoError),
}
