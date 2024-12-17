use crate::container_registry::ContainerRegistry::{
    self, Chainbase, DockerHub, Github, GoogleCloud, Othentic, AWS,
};
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
    AltlayerMach,  // Altlayer Mach AVS
    XterioMach,    // Altlayer Mach AVS
    DodoChainMach, // Altlayer Mach AVS
    CyberMach,     // Altlayer Mach AVS
    GMNetworkMach, // Altlayer Mach AVS
    Omni,
    Automata,
    OpenLayerMainnet,
    OpenLayerHolesky,
    AethosHolesky, // Predicate was Aethos - still live in holesky?
    ArpaChainNode,
    ArpaNetworkNodeClient,
    // OpacityNetwork, //Doesn't really exist yet
    UnifiAVS, // I think this is on-chain only - https://docs.puffer.fi/unifi-avs-protocol
    SkateChainBase, /* Othentic-cli - not sure whats going on here either https://github.com/Skate-Org/avs-X-othentic/blob/main/docker-compose.yml */
    SkateChainMantle, /* Othentic-cli - not sure whats going on here either https://github.com/Skate-Org/avs-X-othentic/blob/main/docker-compose.yml */
    ChainbaseNetworkV1,
    ChainbaseNetworkV2,
    GoPlusAVS,
    UngateInfiniRouteBase,    //Built locally
    UngateInfiniRoutePolygon, // Built locally
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

/* ----------------------------------- */
/* -------- NODE REPOSITORIES -------- */
/* ----------------------------------- */
pub const AVAPROTOCOL_REPO: &str = "avaprotocol/ap-avs";
pub const EIGENDA_REPO: &str = "ghcr.io/layr-labs/eigenda/opr-node";
pub const LAGRANGE_STATECOMS_REPO: &str = "lagrangelabs/lagrange-node";
pub const K3LABS_REPO: &str = "k3official/k3-labs-avs-operator";
pub const EORACLE_REPO: &str = "eoracle/data-validator";
pub const PREDICATE_REPO: &str = "ghcr.io/predicatelabs/operator";
pub const HYPERLANE_REPO: &str = "abacus-labs-dev/hyperlane-agent";
pub const WITNESSCHAIN_REPO: &str = "witnesschain/watchtower";
pub const ALTLAYER_GENERIC_REPO: &str = "altlayer/alt-generic-operator";
pub const ALTLAYER_MACH_REPO: &str = "altlayer/mach-operator";
pub const LAGRANGE_WORKER_REPO: &str = "lagrangelabs/worker";
pub const OMNI_REPO: &str = "omniops/halovisor"; //Holesky only
pub const AUTOMATA_REPO: &str = "automata-network/multi-prover-avs/operator";
pub const OPEN_LAYER_MAINNET_REPO: &str = "openoracle-de73b/operator-js";
pub const OPEN_LAYER_HOLESKY_REPO: &str = "openoracle-de73b/operator-js-holesky";
pub const AETHOS_REPO: &str = "ghcr.io/predicatelabs/operator"; //See above
pub const ARPA_CHAIN_NODE_REPO: &str = "arpachainio/node";
pub const ARPA_NETWORK_NODE_CLIENT_REPO: &str = "ghcr.io/arpa-network/node-client";
pub const CHAINBASE_NETWORK_V1_REPO: &str = "network/chainbase-node";
pub const CHAINBASE_NETWORK_V2_REPO: &str = "network/chainbase-node";
pub const UNGATE_INFINI_ROUTE_BASE_REPO: &str = "infini-route-attestators-public-attester";
pub const UNGATE_INFINI_ROUTE_POLYGON_REPO: &str = "infini-route-attestators-public-attester";

/* ------------------------------------ */
/* ------- NODE CONTAINER NAMES ------- */
/* ------------------------------------ */
//Mainnet:
pub const MACH_AVS_ETHEREUM: &str = "mach-avs-ethereum";
pub const MACH_AVS_ETHEREUM_XTERIO: &str = "mach-avs-ethereum-xterio";
pub const MACH_AVS_ETHEREUM_DODOCHAIN: &str = "mach-avs-ethereum-dodochain";
pub const MACH_AVS_ETHEREUM_CYBER: &str = "mach-avs-ethereum-cyber";
pub const MACH_AVS_ETHEREUM_GMNETWORK: &str = "mach-avs-ethereum-gmnetwork";
pub const EIGENDA_NATIVE_NODE: &str = "eigenda-native-node";
pub const EORACLE_DATA_VALIDATOR: &str = "eoracle-data-validator";
pub const OMNI_HALOVISOR: &str = "halo";
pub const AUTOMATA_OPERATOR: &str = "multi-prover-operator-mainnet";
pub const AVA_OPERATOR: &str = "ap_operator";
pub const CHAINBASE_NETWORK_V1_NODE: &str = "manuscript_node";
pub const CHAINBASE_NETWORK_V2_NODE: &str = "manuscript_node";
pub const GOPLUS_CONTAINER_NAME: &str = "goplus-avs";
pub const UNGATE_MAINNET: &str = "infini-route-attestators-public-mainnet-attester-1";

//Holesky (Will only have a holesky container name if it isn't the same as mainnet):
pub const MACH_AVS_HOLESKY: &str = "mach-avs-holesky";
pub const MACH_AVS_HOLESKY_XTERIO_TESTNET: &str = "mach-avs-holesky-xterio-testnet";
pub const MACH_AVS_HOLESKY_DODOCHAIN: &str = "mach-avs-holesky-dodochain";
pub const MACH_AVS_HOLESKY_CYBER_TESTNET_OPERATOR_NODE: &str =
    "mach-avs-holesky-cyber-testnet-operator-node";
pub const MACH_AVS_HOLESKY_GMNETWORK: &str = "mach-avs-holesky-gmnetwork";
pub const AUTOMATA_OPERATOR_HOLESKY: &str = "multi-prover-operator";
pub const UNGATE_HOLESKY: &str = "infini-route-attestators-public-attester-1";

// We may want to put these methods elsewhere.
impl NodeType {
    pub fn default_repository(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::AvaProtocol => AVAPROTOCOL_REPO,
            Self::EigenDA => EIGENDA_REPO,
            Self::LagrangeStateCommittee => LAGRANGE_STATECOMS_REPO,
            Self::LagrangeZkWorkerHolesky => LAGRANGE_WORKER_REPO,
            Self::LagrangeZkWorkerMainnet => LAGRANGE_WORKER_REPO,
            Self::K3LabsAvs => K3LABS_REPO,
            Self::EOracle => EORACLE_REPO,
            Self::Predicate => PREDICATE_REPO,
            Self::Hyperlane => HYPERLANE_REPO,
            Self::WitnessChain => WITNESSCHAIN_REPO,
            Self::AltlayerMach => ALTLAYER_GENERIC_REPO,
            Self::GMNetworkMach => ALTLAYER_GENERIC_REPO,
            Self::XterioMach => ALTLAYER_MACH_REPO,
            Self::DodoChainMach => ALTLAYER_MACH_REPO,
            Self::CyberMach => ALTLAYER_MACH_REPO,
            Self::Omni => OMNI_REPO,
            Self::Automata => AUTOMATA_REPO,
            Self::OpenLayerMainnet => OPEN_LAYER_MAINNET_REPO,
            Self::OpenLayerHolesky => OPEN_LAYER_HOLESKY_REPO,
            Self::AethosHolesky => PREDICATE_REPO,
            Self::ArpaChainNode => ARPA_CHAIN_NODE_REPO,
            Self::ArpaNetworkNodeClient => ARPA_NETWORK_NODE_CLIENT_REPO,
            Self::ChainbaseNetworkV1 => CHAINBASE_NETWORK_V1_REPO,
            Self::ChainbaseNetworkV2 => CHAINBASE_NETWORK_V2_REPO,
            Self::UngateInfiniRouteBase => UNGATE_INFINI_ROUTE_BASE_REPO,
            Self::UngateInfiniRoutePolygon => UNGATE_INFINI_ROUTE_POLYGON_REPO,
            Self::Brevis => {
                unreachable!("Brevis node type has no repository. This should be unenterable.")
            }
            Self::AlignedLayer => return Err(NodeTypeError::NoRepository),
            Self::PrimevMevCommit => return Err(NodeTypeError::NoRepository),
            Self::GoPlusAVS => return Err(NodeTypeError::NoRepository),
            Self::SkateChainBase => return Err(NodeTypeError::NoRepository),
            Self::SkateChainMantle => return Err(NodeTypeError::NoRepository),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    pub fn registry(&self) -> Result<ContainerRegistry, NodeTypeError> {
        let res = match self {
            Self::EigenDA => Github,
            Self::EOracle => DockerHub,
            Self::AvaProtocol => DockerHub,
            Self::LagrangeStateCommittee => DockerHub,
            Self::LagrangeZkWorkerMainnet => DockerHub,
            Self::LagrangeZkWorkerHolesky => DockerHub,
            Self::K3LabsAvs => DockerHub,
            Self::Predicate => Github,
            Self::Hyperlane => GoogleCloud,
            Self::WitnessChain => DockerHub,
            Self::AltlayerMach => AWS,
            Self::XterioMach => AWS,
            Self::DodoChainMach => AWS,
            Self::CyberMach => AWS,
            Self::GMNetworkMach => AWS,
            Self::Omni => DockerHub,
            Self::Automata => Github,
            Self::OpenLayerMainnet => GoogleCloud,
            Self::OpenLayerHolesky => GoogleCloud,
            Self::AethosHolesky => Github,
            Self::ArpaChainNode => Github,
            Self::ArpaNetworkNodeClient => Github,
            Self::ChainbaseNetworkV1 => Chainbase,
            Self::ChainbaseNetworkV2 => Chainbase,
            Self::UngateInfiniRouteBase => Othentic,
            Self::UngateInfiniRoutePolygon => Othentic,
            Self::GoPlusAVS => Othentic,
            Self::SkateChainBase => Othentic,
            Self::SkateChainMantle => Othentic,
            Self::Brevis => {
                unreachable!("Brevis node type has no docker registry. This should be unenterable.")
            }
            Self::AlignedLayer => return Err(NodeTypeError::NoRegistry),
            Self::PrimevMevCommit => return Err(NodeTypeError::NoRegistry),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    // TODO: Find real default names of nodes marked with `temp_`
    pub fn default_container_name_mainnet(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::EigenDA => EIGENDA_NATIVE_NODE,
            Self::EOracle => EORACLE_DATA_VALIDATOR,
            Self::AltlayerMach => MACH_AVS_ETHEREUM,
            Self::XterioMach => MACH_AVS_ETHEREUM_XTERIO,
            Self::DodoChainMach => MACH_AVS_ETHEREUM_DODOCHAIN,
            Self::CyberMach => MACH_AVS_ETHEREUM_CYBER,
            Self::GMNetworkMach => MACH_AVS_ETHEREUM_GMNETWORK,
            Self::Automata => AUTOMATA_OPERATOR,
            Self::Omni => OMNI_HALOVISOR,
            Self::AvaProtocol => AVA_OPERATOR,
            Self::ChainbaseNetworkV1 => CHAINBASE_NETWORK_V1_NODE,
            Self::LagrangeStateCommittee => todo!(),
            Self::LagrangeZkWorkerMainnet => todo!(),
            Self::K3LabsAvs => todo!(),
            Self::Hyperlane => todo!(),
            Self::WitnessChain => todo!(),
            Self::GoPlusAVS => GOPLUS_CONTAINER_NAME,
            Self::UngateInfiniRouteBase => UNGATE_MAINNET,
            Self::UngateInfiniRoutePolygon => UNGATE_MAINNET,

            Self::Brevis => {
                unreachable!("Brevis node type has no container. This should be unenterable.")
            }
            Self::AlignedLayer => return Err(NodeTypeError::InvalidNodeType),
            Self::PrimevMevCommit => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChainBase => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChainMantle => return Err(NodeTypeError::InvalidNodeType),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::ArpaChainNode => return Err(NodeTypeError::NoDefaultContainerName),
            Self::ArpaNetworkNodeClient => return Err(NodeTypeError::NoDefaultContainerName),
            Self::Predicate => return Err(NodeTypeError::NoDefaultContainerName),
            Self::ChainbaseNetworkV2 => return Err(NodeTypeError::InvalidNodeType),
            Self::LagrangeZkWorkerHolesky => return Err(NodeTypeError::InvalidNodeType),
            Self::OpenLayerHolesky => return Err(NodeTypeError::InvalidNodeType),
            Self::AethosHolesky => return Err(NodeTypeError::InvalidNodeType),
            Self::OpenLayerMainnet => return Err(NodeTypeError::NoDefaultContainerName),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    pub fn default_container_name_holesky(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::EigenDA => EIGENDA_NATIVE_NODE,
            Self::EOracle => EORACLE_DATA_VALIDATOR,
            Self::AltlayerMach => MACH_AVS_HOLESKY,
            Self::XterioMach => MACH_AVS_HOLESKY_XTERIO_TESTNET,
            Self::DodoChainMach => MACH_AVS_HOLESKY_DODOCHAIN,
            Self::CyberMach => MACH_AVS_HOLESKY_CYBER_TESTNET_OPERATOR_NODE,
            Self::GMNetworkMach => MACH_AVS_HOLESKY_GMNETWORK,
            Self::Omni => OMNI_HALOVISOR,
            Self::Automata => AUTOMATA_OPERATOR_HOLESKY,
            Self::AvaProtocol => AVA_OPERATOR,
            Self::ChainbaseNetworkV1 => CHAINBASE_NETWORK_V1_NODE,
            Self::ChainbaseNetworkV2 => CHAINBASE_NETWORK_V2_NODE,
            Self::LagrangeStateCommittee => todo!(),
            Self::LagrangeZkWorkerHolesky => todo!(),
            Self::K3LabsAvs => todo!(),
            Self::Hyperlane => todo!(),
            Self::WitnessChain => todo!(),
            Self::GoPlusAVS => GOPLUS_CONTAINER_NAME,
            Self::UngateInfiniRouteBase => UNGATE_HOLESKY,
            Self::UngateInfiniRoutePolygon => UNGATE_HOLESKY,
            Self::Brevis => {
                unreachable!("Brevis node type has no container. This should be unenterable.")
            }
            Self::AlignedLayer => return Err(NodeTypeError::InvalidNodeType),
            Self::PrimevMevCommit => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChainBase => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChainMantle => return Err(NodeTypeError::InvalidNodeType),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::ArpaChainNode => return Err(NodeTypeError::NoDefaultContainerName),
            Self::ArpaNetworkNodeClient => return Err(NodeTypeError::NoDefaultContainerName),
            Self::Predicate => return Err(NodeTypeError::NoDefaultContainerName),
            Self::AethosHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::OpenLayerHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::OpenLayerMainnet => return Err(NodeTypeError::InvalidNodeType),
            Self::LagrangeZkWorkerMainnet => return Err(NodeTypeError::InvalidNodeType),
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
            // NodeType::AltlayerMach, //AWS rate limits currently
            // NodeType::XterioMach,
            // NodeType::DodoChainMach,
            // NodeType::CyberMach,
            // NodeType::GMNetworkMach,
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
        Self::from_repo(parts[1], parts[0])
    }

    pub fn from_default_container_name(container_name: &str) -> Option<Self> {
        let node_type = match container_name {
            EIGENDA_NATIVE_NODE => Self::EigenDA,
            EORACLE_DATA_VALIDATOR => Self::EOracle,
            MACH_AVS_ETHEREUM => Self::AltlayerMach,
            MACH_AVS_ETHEREUM_XTERIO => Self::XterioMach,
            MACH_AVS_ETHEREUM_DODOCHAIN => Self::DodoChainMach,
            MACH_AVS_ETHEREUM_CYBER => Self::CyberMach,
            MACH_AVS_ETHEREUM_GMNETWORK => Self::GMNetworkMach,
            MACH_AVS_HOLESKY => Self::AltlayerMach,
            MACH_AVS_HOLESKY_XTERIO_TESTNET => Self::XterioMach,
            MACH_AVS_HOLESKY_DODOCHAIN => Self::DodoChainMach,
            MACH_AVS_HOLESKY_CYBER_TESTNET_OPERATOR_NODE => Self::CyberMach,
            MACH_AVS_HOLESKY_GMNETWORK => Self::GMNetworkMach,
            OMNI_HALOVISOR => Self::Omni,
            AUTOMATA_OPERATOR => Self::Automata,
            AUTOMATA_OPERATOR_HOLESKY => Self::Automata,

            _ => return None,
        };
        Some(node_type)
    }

    // Given a repo and tag, get the NodeType, since they have a 1:1 relationship
    pub fn from_repo(repo: &str, tag: &str) -> Option<Self> {
        println!("repo: {}, tag: {}", repo, tag);
        match repo {
            // tag-agnostic nodes
            AVAPROTOCOL_REPO => Some(Self::AvaProtocol),
            EIGENDA_REPO => Some(Self::EigenDA),
            LAGRANGE_STATECOMS_REPO => Some(Self::LagrangeStateCommittee),
            K3LABS_REPO => Some(Self::K3LabsAvs),
            EORACLE_REPO => Some(Self::EOracle),
            PREDICATE_REPO => Some(Self::Predicate),
            HYPERLANE_REPO => Some(Self::Hyperlane),
            WITNESSCHAIN_REPO => Some(Self::WitnessChain),
            // tag-specific nodes
            LAGRANGE_WORKER_REPO => match tag {
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
    #[error("This node type does not have a default container name")]
    NoDefaultContainerName,
    #[error("This node type does not have a repository")]
    NoRepository,
    #[error("This node type does not have a registry")]
    NoRegistry,
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
