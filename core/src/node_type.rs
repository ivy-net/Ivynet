use serde::{Deserialize, Serialize};

// Image names that are used for common AVSes.
const EIGENDA_IMAGE_NAME: &str = "ghcr.io/layr-labs/eigenda/opr-node";
const LAGRANGE_HOLESKY_WORKER_IMAGE_NAME: &str = "lagrangelabs/worker:holesky";

const EIGENDA_METRICS_ID: &str = "da-node";

// const LAGRANGE_MAINNET_WORKER_IMAGE_NAME: &str = "lagrangelabs/worker:mainnet";

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    EigenDA,
    LagrangeHoleskyWorker,
    Unknown,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum NodeTypeError {
    #[error("Invalid node type")]
    InvalidNodeType,
}

impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "eigenda" => NodeType::EigenDA,
            "lagrange:holesky" => NodeType::LagrangeHoleskyWorker,
            _ => panic!("Invalid node type"),
        }
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EigenDA => write!(f, "EigenDA"),
            Self::LagrangeHoleskyWorker => write!(f, "Lagrange Holesky Worker"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

// We may want to put these methods elsewhere.
impl NodeType {
    pub fn default_docker_image_name(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::EigenDA => EIGENDA_IMAGE_NAME,
            Self::LagrangeHoleskyWorker => LAGRANGE_HOLESKY_WORKER_IMAGE_NAME,
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    /// Get a vec of all known node types. Excludes `NodeType::Unknown`.
    pub fn all_known() -> Vec<Self> {
        vec![NodeType::EigenDA, NodeType::LagrangeHoleskyWorker]
    }

    pub fn all_docker_image_names() -> Vec<&'static str> {
        vec![EIGENDA_IMAGE_NAME, LAGRANGE_HOLESKY_WORKER_IMAGE_NAME]
    }

    pub fn from_docker_image_name(image_name: &str) -> Self {
        match image_name {
            EIGENDA_IMAGE_NAME => NodeType::EigenDA,
            LAGRANGE_HOLESKY_WORKER_IMAGE_NAME => Self::LagrangeHoleskyWorker,
            _ => Self::Unknown,
        }
    }
    pub fn from_metrics_name(metrics_id: &str) -> Self {
        match metrics_id {
            EIGENDA_METRICS_ID => Self::EigenDA,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_docker_image_name() {
        assert_eq!(NodeType::from_docker_image_name(EIGENDA_IMAGE_NAME), NodeType::EigenDA);
        assert_eq!(
            NodeType::from_docker_image_name(LAGRANGE_HOLESKY_WORKER_IMAGE_NAME),
            NodeType::LagrangeHoleskyWorker
        );
        assert_eq!(NodeType::from_docker_image_name("invalid"), NodeType::Unknown);
    }
}
