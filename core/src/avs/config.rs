use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

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
                    PathBuf::from(path.to_string())
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

impl NodeConfig {
    pub fn load(path: PathBuf) -> Result<Self, IoError> {
        read_toml(&path)
    }

    pub fn store(&self) {
        write_toml(&self.path(), self).expect("Could not write AVS config");
    }
}

#[derive(ThisError, Debug)]
pub enum NodeConfigError {
    #[error(transparent)]
    ConfigIo(#[from] IoError),
}

pub fn default_config_dir() -> PathBuf {
    dirs::home_dir().expect("Could not get a home directory").join(".ivynet/node_configs")
}
