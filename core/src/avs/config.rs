use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

use crate::{
    env_parser::EnvLineError,
    io::{read_toml, write_toml, IoError},
};

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

    pub fn name(&self) -> String {
        match self {
            NodeConfig::EigenDA(config) => config.name().clone(),
            NodeConfig::Other(config) => {
                if let Some(name) = config.get("name") {
                    name.to_string()
                } else {
                    panic!("No name found in node config")
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
        if !&self.path().exists() {
            std::fs::create_dir_all(self.path().parent().expect("Could not get parent directory"))
                .expect("Could not create config directory");
        }
        write_toml(&self.path(), self).expect("Could not write AVS config");
    }

    pub fn all() -> Result<Vec<Self>, NodeConfigError> {
        let config_dir = default_config_dir();
        let mut configs = vec![];
        for entry in std::fs::read_dir(config_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().unwrap_or_default() == "toml" {
                let config = NodeConfig::load(path)?;
                configs.push(config);
            }
        }
        Ok(configs)
    }
}

impl From<EigenDAConfig> for NodeConfig {
    fn from(config: EigenDAConfig) -> Self {
        NodeConfig::EigenDA(config)
    }
}

#[derive(ThisError, Debug)]
pub enum NodeConfigError {
    #[error(transparent)]
    ConfigIoError(#[from] IoError),
    #[error(transparent)]
    FromHexError(#[from] rustc_hex::FromHexError),
    #[error("transpanret")]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),
    #[error(transparent)]
    KeychainError(#[from] crate::keychain::KeychainError),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("No .env.example file found")]
    NoEnvExample,
    #[error(transparent)]
    EnvLineError(#[from] EnvLineError),
}

pub fn default_config_dir() -> PathBuf {
    dirs::home_dir().expect("Could not get a home directory").join(".ivynet/node_configs")
}
