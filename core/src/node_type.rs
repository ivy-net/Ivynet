use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

const EIGENDA_METRICS_ID: &str = "da-node";

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum NodeType {
    AvaProtocol,
    EigenDA,
    LagrangeStateCommittee,
    LagrangeZkWorkerHolesky,
    LagrangeZkWorkerMainnet,
    K3LabsAvs,
    EOracle,
    Predicate,
    Hyperlane,
    Brevis,
    WitnessChain,
    //New
    AltlayerMach,
    XterioMACH,
    Omni,
    Automata,
    DodoChain,
    OpenLayer,
    CyberMach,
    Aethos,
    ArpaNetwork,
    OpacityNetwork,
    GMNetworkMach,
    UnifiAVS,
    SkateChainBase,
    SkateChainMantle,
    ChainbaseNetworkAVS,
    GoPlusAVS,
    UngateInfiniRouteBase,
    UngateInfiniRoutePolygon,
    PrimevMevCommit,
    AlignedLayer,
    Unknown,
}

// Works with lower case and kebab case - kebab case is what is displayed
impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        let input = s.to_string();

        // Generate different case variations for comparison
        let kebab = input.to_case(Case::Kebab);
        let lower = input.to_case(Case::Lower);
        let pascal = input.to_case(Case::Pascal);
        // Add camel case for compound words
        let camel = input.to_case(Case::Camel);

        // Remove common separators for more flexible matching
        let normalized = input.replace("-", "").replace("_", "").replace(" ", "").to_lowercase();

        NodeType::iter()
            .find(|variant| {
                let variant_str = format!("{:?}", variant);
                let variant_kebab = variant_str.to_case(Case::Kebab);
                let variant_lower = variant_str.to_case(Case::Lower);
                let variant_pascal = variant_str.to_case(Case::Pascal);
                let variant_camel = variant_str.to_case(Case::Camel);
                let variant_normalized =
                    variant_str.replace("-", "").replace("_", "").replace(" ", "").to_lowercase();

                println!("variant_str: {}", variant_str);
                println!("variant_kebab: {}", variant_kebab);
                println!("variant_lower: {}", variant_lower);
                println!("variant_pascal: {}", variant_pascal);
                println!("variant_camel: {}", variant_camel);
                println!("variant_normalized: {}", variant_normalized);

                kebab == variant_kebab ||
                    lower == variant_lower ||
                    pascal == variant_pascal ||
                    camel == variant_camel ||
                    normalized == variant_normalized ||
                    input.eq_ignore_ascii_case(&variant_str)
            })
            .unwrap_or(Self::Unknown)
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Convert enum variant name to kebab case
        let name = format!("{:?}", self).to_case(Case::Kebab);
        write!(f, "{}", name)
    }
}

// We may want to put these methods elsewhere.
impl NodeType {
    pub fn default_repository(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::AvaProtocol => "avaprotocol/ap-avs",
            Self::EigenDA => "ghcr.io/layr-labs/eigenda/opr-node",
            Self::LagrangeStateCommittee => "lagrangelabs/lagrange-node",
            Self::LagrangeZkWorkerHolesky => "lagrangelabs/worker",
            Self::LagrangeZkWorkerMainnet => "lagrangelabs/worker",
            Self::K3LabsAvs => "k3official/k3-labs-avs-operator",
            Self::EOracle => "eoracle/data-validator",
            Self::Predicate => "ghcr.io/predicatelabs/operator",
            Self::Hyperlane => "abacus-labs-dev/hyperlane-agent",
            Self::WitnessChain => "witnesschain/watchtower",
            Self::AltlayerMach => todo!(),
            Self::XterioMACH => todo!(),
            Self::Omni => todo!(),
            Self::Automata => todo!(),
            Self::DodoChain => todo!(),
            Self::OpenLayer => todo!(),
            Self::CyberMach => todo!(),
            Self::Aethos => todo!(),
            Self::ArpaNetwork => todo!(),
            Self::OpacityNetwork => todo!(),
            Self::GMNetworkMach => todo!(),
            Self::UnifiAVS => todo!(),
            Self::SkateChainBase => todo!(),
            Self::SkateChainMantle => todo!(),
            Self::ChainbaseNetworkAVS => todo!(),
            Self::GoPlusAVS => todo!(),
            Self::UngateInfiniRouteBase => todo!(),
            Self::UngateInfiniRoutePolygon => todo!(),
            Self::PrimevMevCommit => todo!(),
            Self::AlignedLayer => todo!(),
            Self::Brevis => {
                unreachable!("Brevis node type has no repository. This should be unenterable.")
            }
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    pub fn registry(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::EigenDA => "ghcr.io",
            Self::EOracle => "registry-1.docker.io",
            Self::AvaProtocol => "registry-1.docker.io",
            Self::LagrangeStateCommittee => "registry-1.docker.io",
            Self::LagrangeZkWorkerMainnet => "registry-1.docker.io",
            Self::LagrangeZkWorkerHolesky => "registry-1.docker.io",
            Self::K3LabsAvs => "registry-1.docker.io",
            Self::Predicate => "ghcr.io",
            Self::Hyperlane => "gcr.io",
            Self::WitnessChain => "registry-1.docker.io",
            Self::AltlayerMach => todo!(),
            Self::XterioMACH => todo!(),
            Self::Omni => todo!(),
            Self::Automata => todo!(),
            Self::DodoChain => todo!(),
            Self::OpenLayer => todo!(),
            Self::CyberMach => todo!(),
            Self::Aethos => todo!(),
            Self::ArpaNetwork => todo!(),
            Self::OpacityNetwork => todo!(),
            Self::GMNetworkMach => todo!(),
            Self::UnifiAVS => todo!(),
            Self::SkateChainBase => todo!(),
            Self::SkateChainMantle => todo!(),
            Self::ChainbaseNetworkAVS => todo!(),
            Self::GoPlusAVS => todo!(),
            Self::UngateInfiniRouteBase => todo!(),
            Self::UngateInfiniRoutePolygon => todo!(),
            Self::PrimevMevCommit => todo!(),
            Self::AlignedLayer => todo!(),
            Self::Brevis => {
                unreachable!("Brevis node type has no docker registry. This should be unenterable.")
            }
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    // TODO: Find real default names of nodes marked with `temp_`
    pub fn default_container_name(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::EigenDA => "eigenda-native-node",
            Self::EOracle => "eoracle-data-validator",
            Self::AvaProtocol => "temp_ap_avs",
            Self::LagrangeStateCommittee => "temp_lagrange-state-committee",
            Self::LagrangeZkWorkerHolesky => "temp_lagrange-zk-worker-holesky",
            Self::LagrangeZkWorkerMainnet => "temp_lagrange-zk-worker-mainnet",
            Self::K3LabsAvs => "temp_k3-labs-avs-operator",
            Self::Predicate => "temp_predicate-operator",
            Self::Hyperlane => "temp_hyperlane-agent",
            Self::WitnessChain => "temp_witnesschain",
            Self::AltlayerMach => todo!(),
            Self::XterioMACH => todo!(),
            Self::Omni => todo!(),
            Self::Automata => todo!(),
            Self::DodoChain => todo!(),
            Self::OpenLayer => todo!(),
            Self::CyberMach => todo!(),
            Self::Aethos => todo!(),
            Self::ArpaNetwork => todo!(),
            Self::OpacityNetwork => todo!(),
            Self::GMNetworkMach => todo!(),
            Self::UnifiAVS => todo!(),
            Self::SkateChainBase => todo!(),
            Self::SkateChainMantle => todo!(),
            Self::ChainbaseNetworkAVS => todo!(),
            Self::GoPlusAVS => todo!(),
            Self::UngateInfiniRouteBase => todo!(),
            Self::UngateInfiniRoutePolygon => todo!(),
            Self::PrimevMevCommit => todo!(),
            Self::AlignedLayer => todo!(),
            Self::Brevis => {
                unreachable!("Brevis node type has no container. This should be unenterable.")
            }
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    /// Get a vec of all known node types. Excludes `NodeType::Unknown`.
    pub fn all_known() -> Vec<Self> {
        vec![
            NodeType::AvaProtocol,
            NodeType::EigenDA,
            NodeType::LagrangeStateCommittee,
            NodeType::LagrangeZkWorkerHolesky,
            NodeType::LagrangeZkWorkerMainnet,
            NodeType::K3LabsAvs,
            NodeType::EOracle,
            NodeType::Predicate,
            NodeType::Hyperlane,
            NodeType::WitnessChain,
            // NodeType::Brevis,
        ]
    }

    pub fn all_default_repositories() -> Vec<&'static str> {
        let all = Self::all_known();
        all.iter().map(|node_type| node_type.default_repository().unwrap()).collect()
    }

    pub fn from_image(image: &str) -> Option<Self> {
        let parts: Vec<&str> = image.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }
        Self::from_repo_tag(parts[1], parts[0])
    }

    pub fn from_repo_tag(repo: &str, tag: &str) -> Option<Self> {
        println!("repo: {}, tag: {}", repo, tag);
        match repo {
            // tag-agnostic nodes
            "avaprotocol/ap-avs" => Some(Self::AvaProtocol),
            "ghcr.io/layr-labs/eigenda/opr-node" => Some(Self::EigenDA),
            "lagrangelabs/lagrange-node" => Some(Self::LagrangeStateCommittee),
            "k3official/k3-labs-avs-operator" => Some(Self::K3LabsAvs),
            "eoracle/data-validator" => Some(Self::EOracle),
            "ghcr.io/predicatelabs/operator" => Some(Self::Predicate),
            "gcr.io/abacus-labs-dev/hyperlane-agent" => Some(Self::Hyperlane),
            "witnesschain/watchtower" => Some(Self::WitnessChain),
            // tag-specific nodes
            "lagrangelabs/worker" => match tag {
                "holesky" => Some(Self::LagrangeZkWorkerHolesky),
                "mainnet" => Some(Self::LagrangeZkWorkerMainnet),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn from_metrics_name(metrics_id: &str) -> Self {
        match metrics_id {
            EIGENDA_METRICS_ID => Self::EigenDA,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum NodeTypeError {
    #[error("Invalid node type")]
    InvalidNodeType,
    #[error("Could not match node type: {0}")]
    NodeMatchError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_docker_image_name() {
        let image_name = "ghcr.io/layr-labs/eigenda/opr-node:0.8.4";
        let node_type = NodeType::from_image(image_name).unwrap();
        assert_eq!(node_type, NodeType::EigenDA);

        let image_name_lagrange_holesky = "lagrangelabs/worker:holesky";
        let node_type_lagrange_holesky = NodeType::from_image(image_name_lagrange_holesky).unwrap();
        assert_eq!(node_type_lagrange_holesky, NodeType::LagrangeZkWorkerHolesky);

        let image_name_lagrange_mainnet = "lagrangelabs/worker:mainnet";
        let node_type_lagrange_mainnet = NodeType::from_image(image_name_lagrange_mainnet).unwrap();
        assert_eq!(node_type_lagrange_mainnet, NodeType::LagrangeZkWorkerMainnet);

        let unknown_image_name = "unknown";
        let unknown_node_type = NodeType::from_image(unknown_image_name);
        assert_eq!(unknown_node_type, None);
    }

    #[test]
    fn test_from_str_kebab_case() {
        let test_cases = vec![
            ("eigen-da", NodeType::EigenDA),
            ("ava-protocol", NodeType::AvaProtocol),
            ("lagrange-state-committee", NodeType::LagrangeStateCommittee),
            ("lagrange-zk-worker-holesky", NodeType::LagrangeZkWorkerHolesky),
            ("e-oracle", NodeType::EOracle),
            ("predicate", NodeType::Predicate),
            ("witness-chain", NodeType::WitnessChain),
        ];

        for (input, expected) in test_cases {
            assert_eq!(NodeType::from(input), expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_from_str_lower_case() {
        let test_cases = vec![
            ("eigenda", NodeType::EigenDA),
            ("avaprotocol", NodeType::AvaProtocol),
            ("lagrangestatecommittee", NodeType::LagrangeStateCommittee),
            ("lagrangezkworkermainnet", NodeType::LagrangeZkWorkerMainnet),
            ("eoracle", NodeType::EOracle),
            ("hyperlane", NodeType::Hyperlane),
        ];

        for (input, expected) in test_cases {
            assert_eq!(NodeType::from(input), expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_from_str_unknown() {
        let test_cases = vec!["not_a_node", "random", "", "123", "unknown-node-type"];

        for input in test_cases {
            assert_eq!(NodeType::from(input), NodeType::Unknown, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_from_str_case_insensitive() {
        let test_cases = vec![
            ("EIGENDA", NodeType::EigenDA),
            ("eigenDA", NodeType::EigenDA),
            ("EigenDa", NodeType::EigenDA),
            ("HYPERLANE", NodeType::Hyperlane),
            ("HyperLane", NodeType::Hyperlane),
        ];

        for (input, expected) in test_cases {
            assert_eq!(NodeType::from(input), expected, "Failed for input: {}", input);
        }
    }
}
