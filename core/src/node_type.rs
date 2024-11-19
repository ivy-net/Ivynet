use serde::{Deserialize, Serialize};

const EIGENDA_IMAGE_NAME: &str = "ghcr.io/layr-labs/eigenda/opr-node";
const LAGRANGE_HOLESKY_WORKER_IMAGE_NAME: &str = "lagrangelabs/worker:holesky";
// const LAGRANGE_MAINNET_WORKER_IMAGE_NAME: &str = "lagrangelabs/worker:mainnet";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    EigenDA,
    LagrangeHoleskyWorker,
    Unknown(String),
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
            other => NodeType::Unknown(other.to_string()),
        }
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::EigenDA => write!(f, "EigenDA"),
            NodeType::LagrangeHoleskyWorker => write!(f, "Lagrange Holesky Worker"),
            NodeType::Unknown(s) => write!(f, "Unknown: {}", s),
        }
    }
}

impl NodeType {
    pub fn default_docker_image_name(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            NodeType::EigenDA => EIGENDA_IMAGE_NAME,
            NodeType::LagrangeHoleskyWorker => LAGRANGE_HOLESKY_WORKER_IMAGE_NAME,
            NodeType::Unknown(_) => return Err(NodeTypeError::InvalidNodeType),
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
            unknown => Ok(NodeType::Unknown(unknown.to_string())),
        }
    }
}
