use std::path::PathBuf;

use url::Url;

use crate::{
    avs::config::{AvsConfig, NodeConfigData},
    bls::Address,
    error::IvyError,
};

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

impl TryFrom<AvsConfig> for EigenDAConfig {
    type Error = IvyError;

    fn try_from(config: AvsConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            path: config.path,
            node_name: config.node_name,
            compose_file: config.compose_file,
            node_data: EigenDANodeData::try_from(config.node_data)?,
        })
    }
}
