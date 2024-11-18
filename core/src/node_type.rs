use serde::{Deserialize, Serialize};

const EIGENDA_IMAGE_NAME: &str = "ghcr.io/layr-labs/eigenda/opr-node";
const LAGRANGE_HOLESKY_WORKER_IMAGE_NAME: &str = "lagrangelabs/worker:holesky";
// const LAGRANGE_MAINNET_WORKER_IMAGE_NAME: &str = "lagrangelabs/worker:mainnet";

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum NodeType {
    EigenDA,
    LagrangeHoleskyWorker,
    Unknown,
}

#[derive(Debug, thiserror::Error)]
pub enum NodeTypeError {
    #[error("Invalid node type")]
    InvalidNodeType,
}

impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "eigenda" => NodeType::EigenDA,
            "lagrange holesky" => NodeType::LagrangeHoleskyWorker,
            _ => panic!("Invalid node type"),
        }
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::EigenDA => write!(f, "EigenDA"),
            NodeType::LagrangeHoleskyWorker => write!(f, "Lagrange Holesky Worker"),
            NodeType::Unknown => write!(f, "Unknown"),
        }
    }
}

// We may want to put these methods elsewhere.
impl NodeType {
    pub fn default_docker_image_name(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            NodeType::EigenDA => EIGENDA_IMAGE_NAME,
            NodeType::LagrangeHoleskyWorker => LAGRANGE_HOLESKY_WORKER_IMAGE_NAME,
            NodeType::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    pub fn all() -> Vec<NodeType> {
        vec![NodeType::EigenDA, NodeType::LagrangeHoleskyWorker]
    }

    pub fn all_docker_image_names() -> Vec<&'static str> {
        vec![EIGENDA_IMAGE_NAME, LAGRANGE_HOLESKY_WORKER_IMAGE_NAME]
    }

    pub fn try_from_docker_image_name(image_name: &str) -> Result<NodeType, NodeTypeError> {
        match image_name {
            EIGENDA_IMAGE_NAME => Ok(NodeType::EigenDA),
            LAGRANGE_HOLESKY_WORKER_IMAGE_NAME => Ok(NodeType::LagrangeHoleskyWorker),
            _ => Err(NodeTypeError::InvalidNodeType),
        }
    }
}
