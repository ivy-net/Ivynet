use ivynet_core::{avs::names::AvsName, ethers::types::Address};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeData {
    pub serial_id: u64,
    pub node_id: Address,
    pub avs_name: AvsName,
    pub avs_version: semver::Version,
    pub active_set: bool,
}
