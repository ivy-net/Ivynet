use ethers::types::Address;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Node {
    pub node_name: String,
    pub node_type: String,
}
