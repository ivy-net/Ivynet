use std::{collections::HashMap, fs::create_dir_all, path::PathBuf};

use ethers::types::{Chain, H160};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
use tracing::info;
use url::Url;

use crate::io::{read_toml, write_toml, IoError};

use super::eigenda::EigenDAConfig;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum NodeType {
    EigenDA,
    Lagrange,
    Unknown,
}

impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "eigenda" => NodeType::EigenDA,
            "lagrange" => NodeType::Lagrange,
            _ => panic!("Invalid node type"),
        }
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::EigenDA => write!(f, "EigenDA"),
            NodeType::Lagrange => write!(f, "Lagrange"),
            NodeType::Unknown => write!(f, "Unknown"),
        }
    }
}

// pub struct NodeConfigBuilder {
//     pub path: Option<PathBuf>,
//     pub node_name: Option<String>,
//     pub node_type: Option<NodeType>,
//     pub compose_file: Option<PathBuf>,
//     pub node_data: HashMap<String, toml::Value>,
// }
//
// impl NodeConfigBuilder {
//     pub fn new() -> Self {
//         Self {
//             path: None,
//             node_name: None,
//             node_type: None,
//             compose_file: None,
//             node_data: HashMap::new(),
//         }
//     }
//     pub fn path(mut self, path: PathBuf) -> Self {
//         self.path = Some(path);
//         self
//     }
//     pub fn node_name(mut self, node_name: String) -> Self {
//         self.node_name = Some(node_name);
//         self
//     }
//     pub fn node_type(mut self, node_type: NodeType) -> Self {
//         self.node_type = Some(node_type);
//         self
//     }
//     pub fn with_data(mut self, key: String, value: impl Into<toml::Value>) -> Self {
//         self.node_data.insert(key, value.into());
//         self
//     }
//     pub fn build(self) -> AvsConfig {
//         AvsConfig {
//             path: self.path.expect("Path is required"),
//             node_name: self.node_name.expect("Node name is required"),
//             node_type: self.node_type.expect("Node type is required"),
//             compose_file: self.compose_file.expect("Compose file is required"),
//             node_data: self.node_data,
//         }
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeConfig {
    EigenDA(EigenDAConfig),
    Other(HashMap<String, toml::Value>),
}

/// TODO: Result for Other type
impl NodeConfig {
    pub fn path(&self) -> PathBuf {
        match self {
            NodeConfig::EigenDA(config) => config.path.clone(),
            NodeConfig::Other(config) => {
                if let Some(path) = config.get("path") {
                    path
                } else {
                    panic!("No path found in node config")
                }
            }
        }
    }

    pub fn node_type(&self) -> NodeType {
        match self {
            NodeConfig::EigenDA(_) => NodeType::EigenDA,
            NodeConfig::Other(_) => NodeType::Unknown,
        }
    }
}

/// Node configuration object, which is used to store and retrieve node configuration data both
/// on-disk and in-memory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvsConfig {
    /// Full path to this object's representation on disk
    pub path: PathBuf,
    /// User-defined name for node identification
    pub node_name: String,
    /// startup methods other than docker-compose, this may be deprecated or moved to node_data.
    pub compose_file: PathBuf,
    /// Node data struct. This is a hashmap to allow for arbitrary data storage, as the node
    /// data structure can vary wildly between node types. This is done in lieu of strong
    /// types, which can be implemented later if necessary.
    pub node_data: NodeConfigData,
}

impl NodeConfig {
    pub fn load(path: PathBuf) -> Result<Self, IoError> {
        read_toml(&path)
    }

    pub fn store(&self) {
        write_toml(&self.path, self).expect("Could not write AVS config");
    }
}

impl AvsConfig {
    // pub fn new(avs_name: &str) -> Self {
    //     Self {
    //         avs_name: String::from(avs_name),
    //         setup_map: HashMap::new(),
    //         //Abstract and empty on purpose - avs should use and modify as needed
    //         avs_settings: HashMap::new(),
    //     }
    // }

    // pub fn load(avs_name: &str) -> Result<Self, AvsConfigError> {
    //     if !AvsConfig::exists(avs_name) {
    //         let configs_path = dirs::home_dir()
    //             .expect("Could not get a home directory")
    //             .join(".ivynet/avs_configs");
    //         create_dir_all(configs_path).expect("Could not create AVS configs directory");
    //         AvsConfig::new(avs_name).store();
    //     }
    //     let avs_config_path = Self::avs_config_path(avs_name);
    //     let avs_config: Self = read_toml(&avs_config_path)?;
    //     Ok(avs_config)
    // }

    // pub fn exists(name: &str) -> bool {
    //     let toml_path = dirs::home_dir()
    //         .expect("Could not get a home directory")
    //         .join(".ivynet/avs_configs")
    //         .join(format!("{}.toml", name));
    //     info!("{}", toml_path.exists());
    //     toml_path.exists()
    // }

    // fn strip_docker_compose(path: PathBuf) -> PathBuf {
    //     let mut path = path;
    //     if path.ends_with("docker-compose.yml") {
    //         path.pop(); // Remove "docker-compose.yml"
    //     }
    //     path
    // }

    // Grabs full path using AVS name from main avs configs directory
    // pub fn avs_config_path(avs_name: &str) -> PathBuf {
    //     dirs::home_dir()
    //         .expect("Could not get a home directory")
    //         .join(".ivynet/avs_configs")
    //         .join(format!("{}.toml", avs_name))
    // }

    // pub fn log_path() -> PathBuf {
    //     dirs::home_dir().expect("Could not get a home directory").join(".ivynet/fluentd/log")
    // }

    // pub fn get_path(&self, chain: Chain) -> PathBuf {
    //     self.setup_map
    //         .get(&chain)
    //         .expect("No path found - please run the setup command")
    //         .path
    //         .clone()
    // }

    // pub fn get_rpc_url(&self, chain: Chain) -> Url {
    //     self.setup_map
    //         .get(&chain)
    //         .expect("No path found - please run the setup command")
    //         .rpc_url
    //         .clone()
    // }

    // pub fn init(
    //     &mut self,
    //     chain: Chain,
    //     rpc_url: Url,
    //     path: PathBuf,
    //     operator_address: H160,
    //     is_custom: bool,
    // ) {
    //     self.setup_map.insert(chain, Setup::new(path, rpc_url, operator_address, is_custom));
    // }

    // pub fn operator_address(&self, chain: Chain) -> H160 {
    //     self.setup_map
    //         .get(&chain)
    //         .expect("No path found - please run the setup command")
    //         .operator_address
    // }
}

#[derive(ThisError, Debug)]
pub enum NodeConfigError {
    #[error(transparent)]
    ConfigIo(#[from] IoError),
}

pub fn default_config_dir() -> PathBuf {
    dirs::home_dir().expect("Could not get a home directory").join(".ivynet/node_configs")
}

// pub fn ask_for_path() -> PathBuf {
//     let path = dialoguer::Input::<String>::new()
//         .with_prompt("Enter the path of the directory containining the AVS's docker-compose.yml")
//         .interact()
//         .expect("Could not get path");
//
//     Self::strip_docker_compose(PathBuf::from(path))
//
//     //TODO: Validate docker-compose.yml exists within directory
// }
