use serde::{Deserialize, Serialize};

// Image names that are used for common AVSes; it's important to note that these are partial names,
// and in production are sometimes appended with versions, such as
// `ghcr.io/layr-labs/eigenda/opr-node:0.8.4`.
const EIGENDA_IMAGE_NAME: &str = "ghcr.io/layr-labs/eigenda/opr-node";
const LAGRANGE_HOLESKY_WORKER_IMAGE_NAME: &str = "lagrangelabs/worker:holesky";
const AVA_IMAGE_NAME: &str = "avaprotocol/ap-avs";

const EIGENDA_METRICS_ID: &str = "da-node";

// const LAGRANGE_MAINNET_WORKER_IMAGE_NAME: &str = "lagrangelabs/worker:mainnet";

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    Ava,
    EigenDA,
    LagrangeHoleskyWorker,
    Unknown,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum NodeTypeError {
    #[error("Invalid node type")]
    InvalidNodeType,
    #[error("Could not match node type: {0}")]
    NodeMatchError(String),
}

impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ava" => NodeType::Ava,
            "eigenda" => NodeType::EigenDA,
            "lagrange:holesky" => NodeType::LagrangeHoleskyWorker,
            _ => NodeType::Unknown,
        }
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ava => write!(f, "Ava"),
            Self::EigenDA => write!(f, "EigenDA"),
            Self::LagrangeHoleskyWorker => write!(f, "Lagrange Holesky Worker"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

// We may want to put these methods elsewhere.
impl NodeType {
    pub fn default_image_name(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::Ava => AVA_IMAGE_NAME,
            Self::EigenDA => EIGENDA_IMAGE_NAME,
            Self::LagrangeHoleskyWorker => LAGRANGE_HOLESKY_WORKER_IMAGE_NAME,
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    pub fn default_container_name(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::Ava => todo!(),
            Self::EigenDA => "eigenda-native-node",
            Self::LagrangeHoleskyWorker => todo!(),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    /// Get a vec of all known node types. Excludes `NodeType::Unknown`.
    pub fn all_known() -> Vec<Self> {
        vec![NodeType::Ava, NodeType::EigenDA, NodeType::LagrangeHoleskyWorker]
    }

    pub fn all_image_names() -> Vec<&'static str> {
        let all = Self::all_known();
        all.iter().map(|node_type| node_type.default_image_name().unwrap()).collect()
    }

    pub fn from_image_name(image_name: &str) -> Self {
        match image_name {
            AVA_IMAGE_NAME => Self::Ava,
            EIGENDA_IMAGE_NAME => Self::EigenDA,
            LAGRANGE_HOLESKY_WORKER_IMAGE_NAME => Self::LagrangeHoleskyWorker,
            _ => Self::Unknown,
        }
    }

    /// Somewhat brittle function for matching in image name to its partial representation
    pub fn from_image_name_partial(image_name: &str) -> Option<Self> {
        let all_image_names = Self::all_image_names();
        for image in all_image_names {
            if image_name.contains(image) {
                return Some(Self::from_image_name(image));
            }
        }
        None
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
        assert_eq!(NodeType::from_image_name(EIGENDA_IMAGE_NAME), NodeType::EigenDA);
        assert_eq!(
            NodeType::from_image_name(LAGRANGE_HOLESKY_WORKER_IMAGE_NAME),
            NodeType::LagrangeHoleskyWorker
        );
        assert_eq!(NodeType::from_image_name("invalid"), NodeType::Unknown);
    }
}
