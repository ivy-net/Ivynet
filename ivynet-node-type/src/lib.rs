use convert_case::{Case, Casing};
use eth_avs::EthereumAvsType;
use eth_node::{EthereumComponentType, EthereumNode};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use tracing::{debug, error, warn};

pub mod eth_avs;
pub mod eth_node;

const EIGENDA_METRICS_ID: &str = "da-node";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeType {
    Unknown,
    EthereumAvs(EthereumAvsType),
    EthereumNode(EthereumNode),
}

impl IntoEnumIterator for NodeType {
    type Iterator = std::vec::IntoIter<NodeType>;

    fn iter() -> Self::Iterator {
        let eth_avs_types = eth_avs::EthereumAvsType::iter();
        let eth_node_types = eth_node::EthereumComponentType::iter();
        let mut types = Vec::new();
        types.extend(eth_avs_types);
        types.extend(eth_node_types);
        types.into_iter();
        todo!()
    }
}

// Works with lower case and kebab case - kebab case is what is displayed
impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        if let Some((prefix, rest)) = s.split_once(':') {
            match prefix {
                "ethavs" => {
                    let avs_type = EthereumAvsType::from_str(rest);
                    if let Some(avs_type) = avs_type {
                        NodeType::EthereumAvs(avs_type)
                    } else {
                        NodeType::Unknown
                    }
                }
                "ethcomp" => {
                    let node_type = EthereumComponentType::from_str(rest);
                    if let Some(node_type) = node_type {
                        NodeType::EthereumNode(node_type)
                    } else {
                        NodeType::Unknown
                    }
                }
                _ => NodeType::Unknown,
            }
        } else {
            NodeType::Unknown
        }
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Convert enum variant name to kebab case
        let name = format!("{:?}", self).to_case(Case::Kebab);
        write!(f, "{}", name)
    }
}

impl Serialize for NodeType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            NodeType::EthereumAvs(inner) => {
                let mut buf = Vec::new();
                inner.serialize(&mut serde_json::Serializer::new(&mut buf))?;
                let inner_str = String::from_utf8(buf).map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(&format!("ethavs:{}", inner_str.trim_matches('"')))
            }
            NodeType::EthereumNode(inner) => {
                let mut buf = Vec::new();
                inner.serialize(&mut serde_json::Serializer::new(&mut buf))?;
                let inner_str = String::from_utf8(buf).map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(&format!("ethnode:{}", inner_str.trim_matches('"')))
            }
            NodeType::Unknown => serializer.serialize_str("unknown"),
        }
    }
}

impl<'de> Deserialize<'de> for NodeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if let Some((prefix, rest)) = s.split_once(':') {
            match prefix {
                "ethavs" => {
                    let avs_type = EthereumAvsType::from_str(rest);
                    if let Some(avs_type) = avs_type {
                        Ok(NodeType::EthereumAvs(avs_type))
                    } else {
                        Err(serde::de::Error::custom("Invalid AVS type"))
                    }
                }
                "ethcomp" => {
                    let node_type = EthereumComponentType::from_str(rest);
                    if let Some(node_type) = node_type {
                        Ok(NodeType::EthereumNode(node_type))
                    } else {
                        Err(serde::de::Error::custom("Invalid node type"))
                    }
                }
                _ => Err(serde::de::Error::custom("Invalid prefix")),
            }
        } else {
            Ok(NodeType::Unknown)
        }
    }
}

impl NodeType {
    /// Get a vec of all known node types. Excludes `NodeType::Unknown`.
    pub fn all_known_with_repo() -> Vec<Self> {
        Self::list_all_variants()
            .into_iter()
            .filter(|node_type| node_type != &Self::Unknown)
            .filter(Self::has_valid_repository)
            .filter(|node_type| node_type.flatten_layered_type())
            .collect()
    }

    pub fn all_default_repositories() -> Vec<&'static str> {
        let all = Self::all_known_with_repo();
        all.iter().map(|node_type| node_type.default_repository().unwrap()).collect()
    }

    pub fn from_image(image: &str) -> Option<Self> {
        let parts: Vec<&str> = image.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            warn!("Unrecognized image format: {}", image);
            return Self::from_repo(parts[0]);
        }
        Self::from_repo(parts[1])
    }

    // Given a repo and tag, get the NodeType, since they have a 1:1 relationship
    pub fn from_repo(repo: &str) -> Option<Self> {
        debug!("repo: {}", repo);
        match repo {
            BLESS_B7S_REPO => Some(Self::BlessB7s),
            ATLAS_NETWORK_REPO => Some(Self::AtlasNetwork),
            AVAPROTOCOL_REPO => Some(Self::AvaProtocol),
            EIGENDA_REPO => Some(Self::EigenDA),
            LAGRANGE_STATECOMS_REPO => Some(Self::LagrangeStateCommittee),
            K3LABS_REPO => Some(Self::K3LabsAvs),
            K3LABS_HOLESKY_REPO => Some(Self::K3LabsAvsHolesky),
            EORACLE_REPO => Some(Self::EOracle),
            PREDICATE_REPO => Some(Self::Predicate),
            HYPERLANE_REPO => Some(Self::Hyperlane(ActiveSet::Unknown)),
            WITNESSCHAIN_REPO => Some(Self::WitnessChain),
            ALTLAYER_GENERIC_REPO => Some(Self::Altlayer(AltlayerType::Unknown)),
            ALTLAYER_MACH_REPO => Some(Self::AltlayerMach(MachType::Unknown)),
            AUTOMATA_REPO => Some(Self::Automata),
            OPEN_LAYER_MAINNET_REPO => Some(Self::OpenLayerMainnet),
            OPEN_LAYER_HOLESKY_REPO => Some(Self::OpenLayerHolesky),
            ARPA_NETWORK_NODE_CLIENT_REPO => Some(Self::ArpaNetworkNodeClient),
            CHAINBASE_NETWORK_V2_REPO => Some(Self::ChainbaseNetwork),
            LAGRANGE_WORKER_REPO => Some(Self::LagrangeZkWorker),
            BREVIS_REPO => Some(Self::Brevis),
            GASP_REPO => Some(Self::Gasp),
            DITTO_NETWORK_REPO => Some(Self::DittoNetwork(ActiveSet::Unknown)),
            NUFFLE_REPO => Some(Self::Nuffle),
            PRIMEV_BIDDER_REPO => Some(Self::PrimevBidder),
            GOPLUS_REPO => Some(Self::GoPlusAVS),
            OMNI_REPO => Some(Self::Omni),
            PRIMUS_REPO => Some(Self::Primus),
            BOLT_REPO => Some(Self::Bolt(ActiveSet::Unknown)),
            CYCLE_REPO => Some(Self::Cycle),
            TANSSI_REPO => Some(Self::Tanssi),
            _ => None,
        }
    }

    pub fn from_default_container_name(container_name: &str) -> Option<Self> {
        let node_type = match container_name {
            ATLAS_NETWORK_CONTAINER_NAME => Self::AtlasNetwork,
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
            CHAINBASE_NETWORK_V1_NODE => Self::ChainbaseNetwork,
            GOPLUS_CONTAINER_NAME => Self::GoPlusAVS,
            LAGRANGE_WORKER_CONTAINER_NAME => Self::LagrangeZkWorker,
            LAGRANGE_STATE_COMMITTEE_CONTAINER_NAME => Self::LagrangeStateCommittee,
            HYPERLANE_AGENT_CONTAINER_NAME => Self::Hyperlane(ActiveSet::Unknown),
            UNGATE_MAINNET => Self::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            UNGATE_NAME_1 => Self::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            UNGATE_NAME_2 => Self::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            UNGATE_NAME_3 => Self::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            GASP_CONTAINER_NAME => Self::Gasp,
            DITTO_NETWORK_CONTAINER_NAME => Self::DittoNetwork(ActiveSet::Unknown),
            NUFFLE_CONTAINER_NAME => Self::Nuffle,
            NUFFLE_CONTAINER_NAME_2 => Self::Nuffle,
            PRIMEV_BIDDER_CONTAINER_NAME => Self::PrimevBidder,
            PRIMUS_CONTAINER_NAME => Self::Primus,
            BOLT_CONTAINER_NAME => Self::Bolt(ActiveSet::Unknown),
            CYCLE_CONTAINER_NAME => Self::Cycle,
            TANSSI_CONTAINER_NAME => Self::Tanssi,
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

    pub fn all_machtypes() -> Vec<Self> {
        MachType::iter().map(NodeType::AltlayerMach).collect()
    }

    pub fn all_altlayertypes() -> Vec<Self> {
        AltlayerType::iter().map(NodeType::Altlayer).collect()
    }

    pub fn all_skatechaintypes() -> Vec<Self> {
        SkateChainType::iter().map(NodeType::SkateChain).collect()
    }

    pub fn all_infiniroutetypes() -> Vec<Self> {
        InfiniRouteType::iter().map(NodeType::UngateInfiniRoute).collect()
    }

    //This function assumes that the repository is in the format of "organization" / "repo"
    //And all of the local builds are just the repo name and no organization (we have control over
    // this bit)
    fn has_valid_repository(&self) -> bool {
        self.default_repository().ok().filter(|repo| repo.split('/').count() > 1).is_some()
    }

    fn flatten_layered_type(&self) -> bool {
        match self {
            NodeType::Altlayer(inner_type) => matches!(inner_type, AltlayerType::Unknown),
            NodeType::AltlayerMach(inner_type) => matches!(inner_type, MachType::Unknown),
            NodeType::SkateChain(inner_type) => matches!(inner_type, SkateChainType::UnknownL2),
            NodeType::UngateInfiniRoute(inner_type) => {
                matches!(inner_type, InfiniRouteType::UnknownL2)
            }
            NodeType::PrimevMevCommit(inner_type) => matches!(inner_type, ActiveSet::Unknown),
            NodeType::Bolt(inner_type) => matches!(inner_type, ActiveSet::Unknown),
            NodeType::Hyperlane(inner_type) => matches!(inner_type, ActiveSet::Unknown),
            NodeType::DittoNetwork(inner_type) => matches!(inner_type, ActiveSet::Unknown),
            _ => true,
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
    #[error("This node type does not have a repository - report to Ivynet team if you believe this is incorrect!")]
    NoRepository,
    #[error("This node type does not have a registry - report to Ivynet team if you believe this is incorrect!")]
    NoRegistry,
    #[error("AVS Specific Error: {0}")]
    SpecializedError(String),
}

#[cfg(test)]
mod node_type_tests {
    use super::*;

    #[test]
    fn test_from_docker_image_name() {
        let no_tag_image_name = "layr-labs/eigenda/opr-node";
        let no_tag_node_type = NodeType::from_image(no_tag_image_name).unwrap();
        assert_eq!(no_tag_node_type, NodeType::EigenDA);

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
            ("altlayer(altlayermach)", NodeType::Altlayer(AltlayerType::AltlayerMach)),
            ("altlayer(gm-network-mach)", NodeType::Altlayer(AltlayerType::GmNetworkMach)),
            ("altlayer-mach(xterio)", NodeType::AltlayerMach(MachType::Xterio)),
            ("altlayer-mach(dodo-chain)", NodeType::AltlayerMach(MachType::DodoChain)),
            ("altlayer-mach(cyber)", NodeType::AltlayerMach(MachType::Cyber)),
            (
                "ungate-infini-route(unknown-l2)",
                NodeType::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            ),
            ("skate-chain(base)", NodeType::SkateChain(SkateChainType::Base)),
            ("skate-chain(mantle)", NodeType::SkateChain(SkateChainType::Mantle)),
            ("skate-chain(unknown-l2)", NodeType::SkateChain(SkateChainType::UnknownL2)),
            ("ditto-network(unknown)", NodeType::DittoNetwork(ActiveSet::Unknown)),
            ("ditto-network(eigenlayer)", NodeType::DittoNetwork(ActiveSet::Eigenlayer)),
            ("ditto-network(symbiotic)", NodeType::DittoNetwork(ActiveSet::Symbiotic)),
            ("bless-b7s", NodeType::BlessB7s),
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
            ("hyperlane(eigenlayer)", NodeType::Hyperlane(ActiveSet::Eigenlayer)),
            ("altlayer(altlayermach)", NodeType::Altlayer(AltlayerType::AltlayerMach)),
            ("altlayer(gmnetworkmach)", NodeType::Altlayer(AltlayerType::GmNetworkMach)),
            ("altlayermach(xterio)", NodeType::AltlayerMach(MachType::Xterio)),
            ("altlayermach(dodochain)", NodeType::AltlayerMach(MachType::DodoChain)),
            ("altlayermach(cyber)", NodeType::AltlayerMach(MachType::Cyber)),
            (
                "ungate-infini-route(unknownl2)",
                NodeType::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            ),
            ("skate-chain(base)", NodeType::SkateChain(SkateChainType::Base)),
            ("skate-chain(mantle)", NodeType::SkateChain(SkateChainType::Mantle)),
            ("skate-chain(unknownl2)", NodeType::SkateChain(SkateChainType::UnknownL2)),
            ("primevmevcommit(eigenlayer)", NodeType::PrimevMevCommit(ActiveSet::Eigenlayer)),
            ("bolt(eigenlayer)", NodeType::Bolt(ActiveSet::Eigenlayer)),
            ("bolt(unknown)", NodeType::Bolt(ActiveSet::Unknown)),
            ("bolt(symbiotic)", NodeType::Bolt(ActiveSet::Symbiotic)),
            ("hyperlane(unknown)", NodeType::Hyperlane(ActiveSet::Unknown)),
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
    fn test_backwards_compatibility() {
        let node_type = NodeType::from("altlayer");
        assert_eq!(node_type, NodeType::Altlayer(AltlayerType::Unknown));
        let node_type = NodeType::from("altlayermach");
        assert_eq!(node_type, NodeType::AltlayerMach(MachType::Unknown));
        let node_type = NodeType::from("bolt");
        assert_eq!(node_type, NodeType::Bolt(ActiveSet::Unknown));
        let node_type = NodeType::from("primev-mev-commit");
        assert_eq!(node_type, NodeType::PrimevMevCommit(ActiveSet::Unknown));
        let node_type = NodeType::from("ungate-infini-route");
        assert_eq!(node_type, NodeType::UngateInfiniRoute(InfiniRouteType::UnknownL2));
        let node_type = NodeType::from("skate-chain");
        assert_eq!(node_type, NodeType::SkateChain(SkateChainType::UnknownL2));
        let node_type = NodeType::from("hyperlane");
        assert_eq!(node_type, NodeType::Hyperlane(ActiveSet::Unknown));
    }

    #[test]
    fn test_from_str_case_insensitive() {
        let test_cases = vec![
            ("EIGENDA", NodeType::EigenDA),
            ("eigenDA", NodeType::EigenDA),
            ("EigenDa", NodeType::EigenDA),
            ("HYPERLANE(UNKNOWN)", NodeType::Hyperlane(ActiveSet::Unknown)),
            ("HyperLane(Unknown)", NodeType::Hyperlane(ActiveSet::Unknown)),
            ("HYPERLANE(EIGENLAYER)", NodeType::Hyperlane(ActiveSet::Eigenlayer)),
            ("HyperLane(Eigenlayer)", NodeType::Hyperlane(ActiveSet::Eigenlayer)),
            ("BLEsSB7S", NodeType::BlessB7s),
        ];

        for (input, expected) in test_cases {
            assert_eq!(NodeType::from(input), expected, "Failed for input: {}", input);
        }
    }
}
