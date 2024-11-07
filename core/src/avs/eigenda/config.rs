use std::path::PathBuf;

use ethers::types::Address;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{avs::config::NodeConfig, error::IvyError};

/// EigenDA node configuration. Mostly a reflection of the AvsConfig struct, with the node_data
/// field pulled out of the NodeConfigData enum for easier access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigenDAConfig {
    pub path: PathBuf,
    pub node_name: String,
    pub compose_file: PathBuf,
    /// Directory containing the EigenDA node resources
    pub node_directory: PathBuf,
    /// Keyfile for the operator
    pub keyfile: PathBuf,
    /// Decrypted operator address,
    pub operator_address: Address,
    /// RPC URL for node connectivity to chain
    pub rpc_url: Url,
}

impl TryFrom<NodeConfig> for EigenDAConfig {
    type Error = IvyError;

    fn try_from(node_config: NodeConfig) -> Result<Self, Self::Error> {
        match node_config {
            NodeConfig::EigenDA(eigenda_config) => Ok(eigenda_config),
            _ => Err(IvyError::ConfigMatchError(
                "EigenDA".to_string(),
                node_config.node_type().to_string(),
            )),
        }
    }
}
