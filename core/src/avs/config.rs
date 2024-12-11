use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

use crate::{
    env_parser::EnvLineError,
    io::{read_toml, write_toml, IoError},
    node_type::NodeType,
};

use super::{eigenda::EigenDAConfig, lagrange::config::LagrangeConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeConfig {
    EigenDA(EigenDAConfig),
    LagrangeZkWorkerHolesky(LagrangeConfig),
    Other(HashMap<String, toml::Value>),
}

/// TODO: Result for Other type
impl NodeConfig {
    pub fn load(path: &PathBuf) -> Result<Self, IoError> {
        read_toml(path)
    }

    pub fn store(&self) {
        if !&self.path().exists() {
            std::fs::create_dir_all(self.path().parent().expect("Could not get parent directory"))
                .expect("Could not create config directory");
        }
        write_toml(&self.path(), self).expect("Could not write AVS config");
    }

    pub fn path(&self) -> PathBuf {
        match self {
            NodeConfig::EigenDA(config) => config.path.clone(),
            NodeConfig::LagrangeZkWorkerHolesky(config) => config.path.clone(),
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
            NodeConfig::EigenDA(config) => config.name(),
            NodeConfig::LagrangeZkWorkerHolesky(config) => config.name(),
            NodeConfig::Other(config) => {
                if let Some(name) = config.get("name") {
                    name.to_string()
                } else {
                    panic!("No name found in node config")
                }
            }
        }
    }

    pub fn all() -> Result<Vec<Self>, NodeConfigError> {
        let config_dir = default_config_dir();
        let mut configs = vec![];
        for entry in std::fs::read_dir(config_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().unwrap_or_default() == "toml" {
                let config = NodeConfig::load(&path)?;
                configs.push(config);
            }
        }
        Ok(configs)
    }

    pub fn node_type(&self) -> NodeType {
        match self {
            NodeConfig::EigenDA(_) => NodeType::EigenDA,
            NodeConfig::LagrangeZkWorkerHolesky(_) => NodeType::LagrangeZkWorkerHolesky,
            //TODO: THE USER NEEDS TO ENTER THE NODE TYPE STRING
            NodeConfig::Other(_) => NodeType::Unknown,
        }
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
    #[error(transparent)]
    KeychainError(#[from] crate::keychain::KeychainError),
    #[error(transparent)]
    DownloadError(#[from] crate::download::DownloadError),
    #[error(transparent)]
    DockerCmdError(#[from] crate::docker::dockercmd::DockerError),
}

pub fn default_config_dir() -> PathBuf {
    dirs::home_dir().expect("Could not get a home directory").join(".ivynet/node_configs")
}

// Node config builder tpy ein progress.
pub struct NodeConfigBuilder {
    pub node_type: NodeType,
}

impl NodeConfigBuilder {
    #[allow(dead_code)]
    fn new(node_type: NodeType) -> Self {
        Self { node_type }
    }
    #[allow(dead_code)]
    fn default_resources_dir(&self) -> PathBuf {
        match self.node_type {
            NodeType::EigenDA => dirs::home_dir()
                .expect("Could not get a home directory")
                .join(".eigenlayer/eigenda"),
            NodeType::LagrangeZkWorkerHolesky => dirs::home_dir()
                .expect("Could not get a home directory")
                .join(".eigenlayer/lagrange"),
            NodeType::LagrangeZkWorkerMainnet => dirs::home_dir()
                .expect("Could not get a home directory")
                .join(".eigenlayer/lagrange"),
            NodeType::Unknown => panic!("Unknown node type"),
            _ => panic!("Unimplementeded node type"),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Password(String);

impl Password {
    pub fn from_dialog(dialog_text: Option<&str>) -> Self {
        let prompt = dialog_text.unwrap_or("Enter password");
        let password = dialoguer::Password::new()
            .with_prompt(prompt)
            .interact()
            .expect("Could not get user input");
        Self(password)
    }
}
