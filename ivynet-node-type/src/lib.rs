use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use tracing::{debug, error, warn};

const EIGENDA_METRICS_ID: &str = "da-node";

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum MachType {
    Xterio,
    DodoChain,
    Cyber,
    Unknown,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum AltlayerType {
    AltlayerMach,
    GmNetworkMach,
    Unknown,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    AvaProtocol,
    EigenDA,
    LagrangeStateCommittee,
    LagrangeZkWorker,
    K3LabsAvs,
    K3LabsAvsHolesky,
    EOracle,
    Predicate,
    Hyperlane,
    Brevis,
    WitnessChain,
    Altlayer(AltlayerType),
    AltlayerMach(MachType),
    Omni,
    Automata,
    OpenLayerMainnet,
    OpenLayerHolesky,
    AethosHolesky, // Predicate was Aethos - still live in holesky?
    ArpaNetworkNodeClient,
    // OpacityNetwork, //Doesn't really exist yet
    UnifiAVS, // I think this is on-chain only - https://docs.puffer.fi/unifi-avs-protocol
    SkateChainBase, /* Othentic-cli - not sure whats going on here either https://github.com/Skate-Org/avs-X-othentic/blob/main/docker-compose.yml */
    SkateChainMantle, /* Othentic-cli - not sure whats going on here either https://github.com/Skate-Org/avs-X-othentic/blob/main/docker-compose.yml */
    ChainbaseNetworkV1,
    ChainbaseNetwork,
    GoPlusAVS,
    UngateInfiniRouteBase,    //Built locally
    UngateInfiniRoutePolygon, // Built locally
    PrimevMevCommit,
    AlignedLayer,
    Unknown,
}

impl IntoEnumIterator for NodeType {
    type Iterator = std::vec::IntoIter<NodeType>;

    fn iter() -> Self::Iterator {
        vec![
            // Simple variants
            NodeType::AvaProtocol,
            NodeType::EigenDA,
            NodeType::LagrangeStateCommittee,
            NodeType::LagrangeZkWorker,
            NodeType::K3LabsAvs,
            NodeType::K3LabsAvsHolesky,
            NodeType::EOracle,
            NodeType::Predicate,
            NodeType::Hyperlane,
            NodeType::Brevis,
            NodeType::WitnessChain,
            NodeType::Omni,
            NodeType::Automata,
            NodeType::OpenLayerMainnet,
            NodeType::OpenLayerHolesky,
            NodeType::AethosHolesky,
            NodeType::ArpaNetworkNodeClient,
            NodeType::UnifiAVS,
            NodeType::SkateChainBase,
            NodeType::SkateChainMantle,
            NodeType::ChainbaseNetworkV1,
            NodeType::ChainbaseNetwork,
            NodeType::GoPlusAVS,
            NodeType::UngateInfiniRouteBase,
            NodeType::UngateInfiniRoutePolygon,
            NodeType::PrimevMevCommit,
            NodeType::AlignedLayer,
            NodeType::Unknown,
        ]
        .into_iter()
        .chain(AltlayerType::iter().map(NodeType::Altlayer))
        .chain(MachType::iter().map(NodeType::AltlayerMach))
        .collect::<Vec<_>>()
        .into_iter()
    }
}

// Works with lower case and kebab case - kebab case is what is displayed
impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        let normalized = s.replace(['-', '_', ' '], "").to_lowercase();

        NodeType::iter()
            .find(|variant| {
                let variant_str = format!("{:?}", variant);
                let variant_normalized = variant_str.replace(['-', '_', ' '], "").to_lowercase();
                normalized == variant_normalized
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
pub const EIGENDA_REPO: &str = "layr-labs/eigenda/opr-node";
pub const LAGRANGE_STATECOMS_REPO: &str = "lagrangelabs/lagrange-node";
pub const K3LABS_REPO: &str = "k3official/k3-labs-avs-operator";
pub const K3LABS_HOLESKY_REPO: &str = "k3official/k3-labs-avs-operator-dev";
pub const EORACLE_REPO: &str = "eoracle/data-validator";
pub const PREDICATE_REPO: &str = "predicatelabs/operator";
pub const HYPERLANE_REPO: &str = "abacus-labs-dev/hyperlane-agent";
pub const WITNESSCHAIN_REPO: &str = "witnesschain/watchtower";
pub const ALTLAYER_GENERIC_REPO: &str = "altlayer/alt-generic-operator";
pub const ALTLAYER_MACH_REPO: &str = "altlayer/mach-operator";
pub const LAGRANGE_WORKER_REPO: &str = "lagrangelabs/worker";
pub const OMNI_REPO: &str = "omniops/halovisor"; //Holesky only
pub const AUTOMATA_REPO: &str = "automata-network/multi-prover-avs/operator";
pub const OPEN_LAYER_MAINNET_REPO: &str = "openoracle-de73b/operator-js";
pub const OPEN_LAYER_HOLESKY_REPO: &str = "openoracle-de73b/operator-js-holesky";
pub const ARPA_NETWORK_NODE_CLIENT_REPO: &str = "arpa-network/node-client";
pub const CHAINBASE_NETWORK_V2_REPO: &str = "network/chainbase-node";

/* ------------------------------------ */
/* ------- NODE CONTAINER NAMES ------- */
/* ------------------------------------ */
//Mainnet:
pub const MACH_AVS_ETHEREUM: &str = "mach-avs-ethereum-generic-operator";
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
pub const WITNESSCHAIN_CONTAINER_NAME: &str = "watchtower";
pub const LAGRANGE_WORKER_CONTAINER_NAME: &str = "worker";
pub const LAGRANGE_STATE_COMMITTEE_CONTAINER_NAME: &str = "lagrange-node";
pub const HYPERLANE_AGENT_CONTAINER_NAME: &str = "ethereum-validator";

//Holesky (Will only have a holesky container name if it isn't the same as mainnet):
pub const MACH_AVS_HOLESKY: &str = "mach-avs-holesky-generic-operator";
pub const MACH_AVS_HOLESKY_XTERIO_TESTNET: &str = "mach-avs-holesky-xterio-testnet";
pub const MACH_AVS_HOLESKY_DODOCHAIN: &str = "mach-avs-holesky-dodochain";
pub const MACH_AVS_HOLESKY_CYBER_TESTNET_OPERATOR_NODE: &str =
    "mach-avs-holesky-cyber-testnet-operator-node";
pub const MACH_AVS_HOLESKY_GMNETWORK: &str = "mach-avs-holesky-gmnetwork";
pub const AUTOMATA_OPERATOR_HOLESKY: &str = "multi-prover-operator";
pub const UNGATE_NAME_1: &str = "infini-route-attestators-public-attester-1";
pub const UNGATE_NAME_2: &str = "infini-route-attestators-public-attester";
pub const UNGATE_NAME_3: &str = "infini-route-attestators-public-attester-webapi";

// We may want to put these methods elsewhere.
impl NodeType {
    pub fn default_repository(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::AvaProtocol => AVAPROTOCOL_REPO,
            Self::EigenDA => EIGENDA_REPO,
            Self::LagrangeStateCommittee => LAGRANGE_STATECOMS_REPO,
            Self::LagrangeZkWorker => LAGRANGE_WORKER_REPO,
            Self::K3LabsAvs => K3LABS_REPO,
            Self::K3LabsAvsHolesky => K3LABS_HOLESKY_REPO,
            Self::EOracle => EORACLE_REPO,
            Self::Predicate => PREDICATE_REPO,
            Self::Hyperlane => HYPERLANE_REPO,
            Self::WitnessChain => WITNESSCHAIN_REPO,
            Self::Altlayer(_altlayer_type) => ALTLAYER_GENERIC_REPO,
            Self::AltlayerMach(_altlayer_mach_type) => ALTLAYER_MACH_REPO,
            Self::Omni => OMNI_REPO,
            Self::Automata => AUTOMATA_REPO,
            Self::OpenLayerMainnet => OPEN_LAYER_MAINNET_REPO,
            Self::OpenLayerHolesky => OPEN_LAYER_HOLESKY_REPO,
            Self::ArpaNetworkNodeClient => ARPA_NETWORK_NODE_CLIENT_REPO,
            Self::ChainbaseNetwork => CHAINBASE_NETWORK_V2_REPO,
            Self::Brevis => {
                return Err(NodeTypeError::SpecializedError("Brevis is executable only".to_string()))
            }
            Self::AethosHolesky => {
                return Err(NodeTypeError::SpecializedError(
                    "AethosHolesky is deprecated - now predicate".to_string(),
                ))
            }
            Self::ChainbaseNetworkV1 => {
                return Err(NodeTypeError::SpecializedError(
                    "ChainbaseNetworkV1 is deprecated - update to V2 - ChainbaseNetwork"
                        .to_string(),
                ))
            }
            Self::UngateInfiniRouteBase => return Err(NodeTypeError::NoRepository),
            Self::UngateInfiniRoutePolygon => return Err(NodeTypeError::NoRepository),
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

    // TODO: Find real default names of nodes marked with `temp_`
    pub fn default_container_name_mainnet(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::EigenDA => EIGENDA_NATIVE_NODE,
            Self::EOracle => EORACLE_DATA_VALIDATOR,
            Self::Automata => AUTOMATA_OPERATOR,
            Self::Omni => OMNI_HALOVISOR,
            Self::AvaProtocol => AVA_OPERATOR,
            Self::ChainbaseNetwork => CHAINBASE_NETWORK_V2_NODE,
            Self::LagrangeStateCommittee => LAGRANGE_STATE_COMMITTEE_CONTAINER_NAME,
            Self::LagrangeZkWorker => LAGRANGE_WORKER_CONTAINER_NAME,
            Self::Hyperlane => HYPERLANE_AGENT_CONTAINER_NAME,
            Self::WitnessChain => WITNESSCHAIN_CONTAINER_NAME,
            Self::GoPlusAVS => GOPLUS_CONTAINER_NAME,
            Self::UngateInfiniRouteBase => UNGATE_MAINNET,
            Self::UngateInfiniRoutePolygon => UNGATE_MAINNET,
            Self::Brevis => {
                return Err(NodeTypeError::SpecializedError("Brevis is executable only".to_string()))
            }
            Self::Altlayer(altlayer_type) => {
                match altlayer_type {
                    AltlayerType::AltlayerMach => MACH_AVS_ETHEREUM,
                    AltlayerType::GmNetworkMach => MACH_AVS_ETHEREUM_GMNETWORK,
                    AltlayerType::Unknown => return Err(NodeTypeError::SpecializedError("This unknown altlayer type isn't an actual container, its just the image. Assign a specific altlayer type".to_string())),
                }
            },
            Self::AltlayerMach(altlayer_mach_type) => {
                match altlayer_mach_type {
                    MachType::Xterio => MACH_AVS_ETHEREUM_XTERIO,
                    MachType::DodoChain => MACH_AVS_ETHEREUM_DODOCHAIN,
                    MachType::Cyber => MACH_AVS_ETHEREUM_CYBER,
                    MachType::Unknown => return Err(NodeTypeError::SpecializedError("GenericAltlayer isn't an actual container, its just the image. Assign a specific altlayer type".to_string())),
                }
            },
            Self::K3LabsAvs => return Err(NodeTypeError::NoDefaultContainerName),
            Self::K3LabsAvsHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::AlignedLayer => return Err(NodeTypeError::InvalidNodeType),
            Self::PrimevMevCommit => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChainBase => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChainMantle => return Err(NodeTypeError::InvalidNodeType),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::ArpaNetworkNodeClient => return Err(NodeTypeError::NoDefaultContainerName),
            Self::Predicate => return Err(NodeTypeError::NoDefaultContainerName),
            Self::ChainbaseNetworkV1 => {
                return Err(NodeTypeError::SpecializedError(
                    "ChainbaseNetworkV1 is deprecated - update to V2 - ChainbaseNetwork"
                        .to_string(),
                ))
            }
            Self::OpenLayerHolesky => return Err(NodeTypeError::InvalidNodeType),
            Self::AethosHolesky => {
                return Err(NodeTypeError::SpecializedError(
                    "AethosHolesky is deprecated - now Predicate".to_string(),
                ))
            }
            Self::OpenLayerMainnet => return Err(NodeTypeError::NoDefaultContainerName),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    pub fn default_container_name_holesky(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::EigenDA => EIGENDA_NATIVE_NODE,
            Self::EOracle => EORACLE_DATA_VALIDATOR,
            Self::Altlayer(altlayer_type) => {
                match altlayer_type {
                    AltlayerType::AltlayerMach => MACH_AVS_HOLESKY,
                    AltlayerType::GmNetworkMach => MACH_AVS_HOLESKY_GMNETWORK,
                    AltlayerType::Unknown => return Err(NodeTypeError::SpecializedError("This unknown altlayer type isn't an actual container, its just the image. Assign a specific altlayer type".to_string())),
                }
            },
            Self::AltlayerMach(altlayer_mach_type) => {
                match altlayer_mach_type {
                    MachType::Xterio => MACH_AVS_HOLESKY_XTERIO_TESTNET,
                    MachType::DodoChain => MACH_AVS_HOLESKY_DODOCHAIN,
                    MachType::Cyber => MACH_AVS_HOLESKY_CYBER_TESTNET_OPERATOR_NODE,
                    MachType::Unknown => return Err(NodeTypeError::SpecializedError("GenericAltlayer isn't an actual container, its just the image. Assign a specific altlayer type".to_string())),
                }
            },
            Self::Omni => OMNI_HALOVISOR,
            Self::Automata => AUTOMATA_OPERATOR_HOLESKY,
            Self::AvaProtocol => AVA_OPERATOR,
            Self::ChainbaseNetwork => CHAINBASE_NETWORK_V2_NODE,
            Self::LagrangeStateCommittee => LAGRANGE_STATE_COMMITTEE_CONTAINER_NAME,
            Self::LagrangeZkWorker => LAGRANGE_WORKER_CONTAINER_NAME,
            Self::Hyperlane => HYPERLANE_AGENT_CONTAINER_NAME,
            Self::WitnessChain => WITNESSCHAIN_CONTAINER_NAME,
            Self::GoPlusAVS => GOPLUS_CONTAINER_NAME,
            Self::UngateInfiniRouteBase => UNGATE_NAME_1,
            Self::UngateInfiniRoutePolygon => UNGATE_NAME_1,
            Self::Brevis => {
                return Err(NodeTypeError::SpecializedError("Brevis is executable only".to_string()))
            }
            Self::ChainbaseNetworkV1 => {
                return Err(NodeTypeError::SpecializedError(
                    "ChainbaseNetworkV1 is deprecated - update to V2 - ChainbaseNetwork"
                        .to_string(),
                ))
            }
            Self::K3LabsAvs => return Err(NodeTypeError::NoDefaultContainerName),
            Self::K3LabsAvsHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::AlignedLayer => return Err(NodeTypeError::InvalidNodeType),
            Self::PrimevMevCommit => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChainBase => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChainMantle => return Err(NodeTypeError::InvalidNodeType),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::ArpaNetworkNodeClient => return Err(NodeTypeError::NoDefaultContainerName),
            Self::Predicate => return Err(NodeTypeError::NoDefaultContainerName),
            Self::AethosHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::OpenLayerHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::OpenLayerMainnet => return Err(NodeTypeError::InvalidNodeType),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }

    /// Get a vec of all known node types. Excludes `NodeType::Unknown`.
    pub fn all_known_with_repo() -> Vec<Self> {
        vec![
            NodeType::AvaProtocol,
            NodeType::EigenDA,
            NodeType::LagrangeStateCommittee,
            NodeType::LagrangeZkWorker,
            NodeType::K3LabsAvs,
            NodeType::K3LabsAvsHolesky,
            NodeType::EOracle,
            NodeType::Predicate,
            NodeType::Hyperlane,
            NodeType::WitnessChain,
            NodeType::Omni,
            NodeType::Automata,
            NodeType::OpenLayerMainnet,
            NodeType::OpenLayerHolesky,
            NodeType::ArpaNetworkNodeClient,
            NodeType::ChainbaseNetwork,
            //AWS rate limits currently
            NodeType::Altlayer(AltlayerType::Unknown),
            NodeType::AltlayerMach(MachType::Unknown),
        ]
    }

    pub fn all_default_repositories() -> Vec<&'static str> {
        let all = Self::all_known_with_repo();
        all.iter().map(|node_type| node_type.default_repository().unwrap()).collect()
    }

    pub fn from_image(image: &str) -> Option<Self> {
        let parts: Vec<&str> = image.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            warn!("Unrecognized image format: {}", image);
            return None;
        }
        Self::from_repo(parts[1], parts[0])
    }

    // Given a repo and tag, get the NodeType, since they have a 1:1 relationship
    pub fn from_repo(repo: &str, tag: &str) -> Option<Self> {
        debug!("repo: {}, tag: {}", repo, tag);
        match repo {
            // tag-agnostic nodes
            AVAPROTOCOL_REPO => Some(Self::AvaProtocol),
            EIGENDA_REPO => Some(Self::EigenDA),
            LAGRANGE_STATECOMS_REPO => Some(Self::LagrangeStateCommittee),
            K3LABS_REPO => Some(Self::K3LabsAvs),
            K3LABS_HOLESKY_REPO => Some(Self::K3LabsAvsHolesky),
            EORACLE_REPO => Some(Self::EOracle),
            PREDICATE_REPO => Some(Self::Predicate),
            HYPERLANE_REPO => Some(Self::Hyperlane),
            WITNESSCHAIN_REPO => Some(Self::WitnessChain),
            ALTLAYER_GENERIC_REPO => Some(Self::Altlayer(AltlayerType::Unknown)),
            ALTLAYER_MACH_REPO => Some(Self::AltlayerMach(MachType::Unknown)),
            AUTOMATA_REPO => Some(Self::Automata),
            OPEN_LAYER_MAINNET_REPO => Some(Self::OpenLayerMainnet),
            OPEN_LAYER_HOLESKY_REPO => Some(Self::OpenLayerHolesky),
            ARPA_NETWORK_NODE_CLIENT_REPO => Some(Self::ArpaNetworkNodeClient),
            CHAINBASE_NETWORK_V2_REPO => Some(Self::ChainbaseNetwork),
            LAGRANGE_WORKER_REPO => Some(Self::LagrangeZkWorker),
            _ => None,
        }
    }

    pub fn from_default_container_name(container_name: &str) -> Option<Self> {
        let node_type = match container_name {
            EIGENDA_NATIVE_NODE => Self::EigenDA,
            EORACLE_DATA_VALIDATOR => Self::EOracle,
            MACH_AVS_ETHEREUM => Self::Altlayer(AltlayerType::AltlayerMach),
            MACH_AVS_ETHEREUM_GMNETWORK => Self::Altlayer(AltlayerType::GmNetworkMach),
            MACH_AVS_HOLESKY => Self::Altlayer(AltlayerType::AltlayerMach),
            MACH_AVS_HOLESKY_GMNETWORK => Self::Altlayer(AltlayerType::GmNetworkMach),
            MACH_AVS_ETHEREUM_XTERIO => Self::AltlayerMach(MachType::Xterio),
            MACH_AVS_ETHEREUM_DODOCHAIN => Self::AltlayerMach(MachType::DodoChain),
            MACH_AVS_ETHEREUM_CYBER => Self::AltlayerMach(MachType::Cyber),
            MACH_AVS_HOLESKY_XTERIO_TESTNET => Self::AltlayerMach(MachType::Xterio),
            MACH_AVS_HOLESKY_DODOCHAIN => Self::AltlayerMach(MachType::DodoChain),
            MACH_AVS_HOLESKY_CYBER_TESTNET_OPERATOR_NODE => Self::AltlayerMach(MachType::Cyber),
            OMNI_HALOVISOR => Self::Omni,
            AUTOMATA_OPERATOR => Self::Automata,
            AUTOMATA_OPERATOR_HOLESKY => Self::Automata,
            AVA_OPERATOR => Self::AvaProtocol,
            CHAINBASE_NETWORK_V1_NODE => Self::ChainbaseNetworkV1,
            GOPLUS_CONTAINER_NAME => Self::GoPlusAVS,
            UNGATE_MAINNET => Self::UngateInfiniRouteBase,
            WITNESSCHAIN_CONTAINER_NAME => Self::WitnessChain,
            LAGRANGE_WORKER_CONTAINER_NAME => Self::LagrangeZkWorker,
            LAGRANGE_STATE_COMMITTEE_CONTAINER_NAME => Self::LagrangeStateCommittee,
            HYPERLANE_AGENT_CONTAINER_NAME => Self::Hyperlane,
            UNGATE_NAME_1 => Self::UngateInfiniRouteBase,
            UNGATE_NAME_2 => Self::UngateInfiniRouteBase,
            UNGATE_NAME_3 => Self::UngateInfiniRouteBase,
            _ => return None,
        };
        Some(node_type)
    }

    pub fn from_metrics_name(metrics_id: &str) -> Self {
        match metrics_id {
            EIGENDA_METRICS_ID => Self::EigenDA,
            _ => Self::Unknown,
        }
    }

    pub fn list_all_variants() -> Vec<Self> {
        Self::iter().collect()
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
    #[error("AVS Specific Error: {0}")]
    SpecializedError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_docker_image_name() {
        let image_name = "layr-labs/eigenda/opr-node:0.8.4";
        let node_type = NodeType::from_image(image_name).unwrap();
        assert_eq!(node_type, NodeType::EigenDA);

        let image_name_lagrange_holesky = "lagrangelabs/worker:holesky";
        let node_type_lagrange_holesky = NodeType::from_image(image_name_lagrange_holesky).unwrap();
        assert_eq!(node_type_lagrange_holesky, NodeType::LagrangeZkWorker);

        let image_name_lagrange_mainnet = "lagrangelabs/worker:mainnet";
        let node_type_lagrange_mainnet = NodeType::from_image(image_name_lagrange_mainnet).unwrap();
        assert_eq!(node_type_lagrange_mainnet, NodeType::LagrangeZkWorker);

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
            ("lagrange-zk-worker", NodeType::LagrangeZkWorker),
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
            ("lagrangezkworker", NodeType::LagrangeZkWorker),
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
