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

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum InfiniRouteType {
    Base,
    Polygon,
    UnknownL2,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum SkateChainType {
    Base,
    Mantle,
    UnknownL2,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeType {
    AvaProtocol,
    EigenDA,
    LagrangeStateCommittee,
    LagrangeZkWorker,
    LagrangeZKProver,
    K3LabsAvs,
    K3LabsAvsHolesky,
    EOracle,
    Gasp,
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
    ChainbaseNetworkV1,
    SkateChain(SkateChainType), /* Othentic-cli - not sure whats going on here either https://github.com/Skate-Org/avs-X-othentic/blob/main/docker-compose.yml */
    ChainbaseNetwork,
    GoPlusAVS,
    UngateInfiniRoute(InfiniRouteType), //Built locally
    PrimevMevCommit,
    AlignedLayer,
    Unknown,
    DittoNetwork,
    Nuffle,
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
            NodeType::ChainbaseNetwork,
            NodeType::GoPlusAVS,
            NodeType::PrimevMevCommit,
            NodeType::AlignedLayer,
            NodeType::DittoNetwork,
            NodeType::Gasp,
            NodeType::Nuffle,
            NodeType::Unknown,
        ]
        .into_iter()
        .chain(AltlayerType::iter().map(NodeType::Altlayer))
        .chain(MachType::iter().map(NodeType::AltlayerMach))
        .chain(SkateChainType::iter().map(NodeType::SkateChain))
        .chain(InfiniRouteType::iter().map(NodeType::UngateInfiniRoute))
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

impl Serialize for NodeType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use convert_case::{Case, Casing};

        match self {
            // Compound types - need special formatting
            NodeType::Altlayer(inner) => {
                let outer = "altlayer".to_string();
                let inner_str = serde_json::to_string(inner)
                    .map_err(serde::ser::Error::custom)?
                    .trim_matches('"')
                    .to_case(Case::Kebab);
                serializer.serialize_str(&format!("{}({})", outer, inner_str))
            }
            NodeType::AltlayerMach(inner) => {
                let outer = "altlayer-mach".to_string();
                let inner_str = serde_json::to_string(inner)
                    .map_err(serde::ser::Error::custom)?
                    .trim_matches('"')
                    .to_case(Case::Kebab);
                serializer.serialize_str(&format!("{}({})", outer, inner_str))
            }
            NodeType::SkateChain(inner) => {
                let outer = "skate-chain".to_string();
                let inner_str = serde_json::to_string(inner)
                    .map_err(serde::ser::Error::custom)?
                    .trim_matches('"')
                    .to_case(Case::Kebab);
                serializer.serialize_str(&format!("{}({})", outer, inner_str))
            }
            NodeType::UngateInfiniRoute(inner) => {
                let outer = "ungate-infini-route".to_string();
                let inner_str = serde_json::to_string(inner)
                    .map_err(serde::ser::Error::custom)?
                    .trim_matches('"')
                    .to_case(Case::Kebab);
                serializer.serialize_str(&format!("{}({})", outer, inner_str))
            }
            // Simple types - use Display implementation
            _ => serializer.serialize_str(&self.to_string()),
        }
    }
}

impl<'de> Deserialize<'de> for NodeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let s = String::deserialize(deserializer)?;

        if let Some(outer_inner) = s.split_once('(') {
            let outer = outer_inner.0;
            let inner = outer_inner.1.trim_end_matches(')');

            // Convert to normalized form (lowercase, no hyphens)
            let normalized_outer = outer.replace(['-', '_', ' '], "").to_lowercase();

            // Match on outer type and deserialize inner using derived implementation
            match normalized_outer.as_str() {
                "altlayer" => {
                    let inner_type: AltlayerType = serde_json::from_str(&format!("\"{}\"", inner))
                        .map_err(D::Error::custom)?;
                    Ok(NodeType::Altlayer(inner_type))
                }
                "altlayermach" => {
                    let inner_type: MachType = serde_json::from_str(&format!("\"{}\"", inner))
                        .map_err(D::Error::custom)?;
                    Ok(NodeType::AltlayerMach(inner_type))
                }
                "skatechain" => {
                    let inner_type: SkateChainType =
                        serde_json::from_str(&format!("\"{}\"", inner))
                            .map_err(D::Error::custom)?;
                    Ok(NodeType::SkateChain(inner_type))
                }
                "ungateinfiniroute" => {
                    let inner_type: InfiniRouteType =
                        serde_json::from_str(&format!("\"{}\"", inner))
                            .map_err(D::Error::custom)?;
                    Ok(NodeType::UngateInfiniRoute(inner_type))
                }
                _ => Err(D::Error::custom("Invalid compound NodeType")),
            }
        } else {
            // Fall back to existing From<&str> implementation for simple types
            Ok(NodeType::from(s.as_str()))
        }
    }
}

/* ----------------------------------- */
/* -------- NODE IMAGE NAMES -------- */
/* ----------------------------------- */
pub const AVAPROTOCOL_REPO: &str = "avaprotocol/ap-avs";
pub const EIGENDA_REPO: &str = "layr-labs/eigenda/opr-node";
pub const LAGRANGE_STATECOMS_REPO: &str = "lagrangelabs/lagrange-node";
pub const LAGRANGE_WORKER_REPO: &str = "lagrangelabs/worker";
pub const LAGRANGE_ZKPROVER_REPO: &str = "lagrangelabs/lpn-zksync-prover";
pub const K3LABS_REPO: &str = "k3official/k3-labs-avs-operator";
pub const K3LABS_HOLESKY_REPO: &str = "k3official/k3-labs-avs-operator-dev";
pub const EORACLE_REPO: &str = "eoracle/data-validator";
pub const PREDICATE_REPO: &str = "predicatelabs/operator";
pub const HYPERLANE_REPO: &str = "abacus-labs-dev/hyperlane-agent";
pub const WITNESSCHAIN_REPO: &str = "witnesschain/watchtower";
pub const ALTLAYER_GENERIC_REPO: &str = "altlayer/alt-generic-operator";
pub const ALTLAYER_MACH_REPO: &str = "altlayer/mach-operator";
pub const AUTOMATA_REPO: &str = "automata-network/multi-prover-avs/operator";
pub const OPEN_LAYER_MAINNET_REPO: &str = "openoracle-de73b/operator-js";
pub const OPEN_LAYER_HOLESKY_REPO: &str = "openoracle-de73b/operator-js-holesky";
pub const ARPA_NETWORK_NODE_CLIENT_REPO: &str = "arpa-network/node-client";
pub const CHAINBASE_NETWORK_V2_REPO: &str = "network/chainbase-node";
pub const BREVIS_REPO: &str = "brevis-avs"; //Local only
pub const GOPLUS_REPO: &str = "goplus_avs"; //Local only
pub const NUFFLE_REPO: &str = "nffl-operator"; //Local only // Holesky Only
pub const GASP_REPO: &str = "gaspxyz/gasp-avs"; //Holesky only
pub const DITTO_NETWORK_REPO: &str = "dittonetwork/avs-operator"; //Holesky only
pub const PRIMEV_LOCAL_REPO: &str = "bidder_node_docker-mev-commit-bidderr"; //Local only
pub const PRIMEV_IMAGE_REPO: &str = "primevprotocol/mev-commit"; //Remote only //I think its out of date?
pub const OMNI_REPO: &str = "omniops/halovisor"; //Holesky only

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
pub const WITNESSCHAIN_CONTAINER_NAME: &str = "watchtower"; //Lagrange and witnesschain now both use watchtower
pub const LAGRANGE_WORKER_CONTAINER_NAME: &str = "worker";
pub const LAGRANGE_STATE_COMMITTEE_CONTAINER_NAME: &str = "lagrange-node";
pub const HYPERLANE_AGENT_CONTAINER_NAME: &str = "ethereum-validator";
pub const GASP_CONTAINER_NAME: &str = "gasp-avs";
pub const DITTO_NETWORK_CONTAINER_NAME: &str = "ditto-operator";
pub const PRIMEV_MEV_COMMIT_CONTAINER_NAME: &str = "mev-commit-bidder-1";

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
pub const NUFFLE_CONTAINER_NAME: &str = "nffl-operator0";
pub const NUFFLE_CONTAINER_NAME_2: &str = "nffl-operator1";

// We may want to put these methods elsewhere.
impl NodeType {
    pub fn default_repository(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::Gasp => GASP_REPO,
            Self::AvaProtocol => AVAPROTOCOL_REPO,
            Self::EigenDA => EIGENDA_REPO,
            Self::LagrangeStateCommittee => LAGRANGE_STATECOMS_REPO,
            Self::LagrangeZkWorker => LAGRANGE_WORKER_REPO,
            Self::LagrangeZKProver => LAGRANGE_ZKPROVER_REPO,
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
            Self::PrimevMevCommit => PRIMEV_LOCAL_REPO,
            Self::Brevis => return Err(NodeTypeError::NoRepository),
            Self::GoPlusAVS => GOPLUS_REPO,
            Self::DittoNetwork => DITTO_NETWORK_REPO,
            Self::Nuffle => return Err(NodeTypeError::NoRepository),
            Self::UngateInfiniRoute(_infini_route_type) => return Err(NodeTypeError::NoRepository),
            Self::AlignedLayer => return Err(NodeTypeError::NoRepository),
            Self::SkateChain(_skate_chain_type) => return Err(NodeTypeError::NoRepository),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
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
        };
        Ok(res)
    }

    // TODO: Find real default names of nodes marked with `temp_`
    pub fn default_container_name_mainnet(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::Gasp => GASP_CONTAINER_NAME,
            Self::EigenDA => EIGENDA_NATIVE_NODE,
            Self::EOracle => EORACLE_DATA_VALIDATOR,
            Self::Automata => AUTOMATA_OPERATOR,
            Self::Omni => OMNI_HALOVISOR,
            Self::AvaProtocol => AVA_OPERATOR,
            Self::ChainbaseNetwork => CHAINBASE_NETWORK_V2_NODE,
            Self::LagrangeStateCommittee => LAGRANGE_STATE_COMMITTEE_CONTAINER_NAME,
            Self::LagrangeZkWorker => LAGRANGE_WORKER_CONTAINER_NAME,
            Self::LagrangeZKProver => {
                return Err(NodeTypeError::SpecializedError(
                    "TODO:".to_string(),
                ))
            }
            Self::Hyperlane => HYPERLANE_AGENT_CONTAINER_NAME,
            Self::WitnessChain => WITNESSCHAIN_CONTAINER_NAME,
            Self::GoPlusAVS => GOPLUS_CONTAINER_NAME,
            Self::UngateInfiniRoute(_infini_route_type) => UNGATE_MAINNET,
            Self::DittoNetwork => DITTO_NETWORK_CONTAINER_NAME,
            Self::PrimevMevCommit => PRIMEV_MEV_COMMIT_CONTAINER_NAME,
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
            Self::Brevis => return Err(NodeTypeError::NoDefaultContainerName),
            Self::K3LabsAvs => return Err(NodeTypeError::NoDefaultContainerName),
            Self::K3LabsAvsHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::AlignedLayer => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChain(_skate_chain_type) => return Err(NodeTypeError::NoDefaultContainerName),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::ArpaNetworkNodeClient => return Err(NodeTypeError::NoDefaultContainerName),
            Self::Predicate => return Err(NodeTypeError::NoDefaultContainerName),
            Self::OpenLayerMainnet => return Err(NodeTypeError::NoDefaultContainerName),
            Self::OpenLayerHolesky => return Err(NodeTypeError::InvalidNodeType),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
            Self::Nuffle => {
                return Err(NodeTypeError::SpecializedError(
                    "Not on mainnet"
                        .to_string(),
                ))
            }
            Self::ChainbaseNetworkV1 => {
                return Err(NodeTypeError::SpecializedError(
                    "ChainbaseNetworkV1 is deprecated - update to V2 - ChainbaseNetwork"
                        .to_string(),
                ))
            }

            Self::AethosHolesky => {
                return Err(NodeTypeError::SpecializedError(
                    "AethosHolesky is deprecated - now Predicate".to_string(),
                ))
            }
        };
        Ok(res)
    }

    pub fn default_container_name_holesky(&self) -> Result<&'static str, NodeTypeError> {
        let res = match self {
            Self::Gasp => GASP_CONTAINER_NAME,
            Self::EigenDA => EIGENDA_NATIVE_NODE,
            Self::EOracle => EORACLE_DATA_VALIDATOR,
            Self::DittoNetwork => DITTO_NETWORK_CONTAINER_NAME,
            Self::Omni => OMNI_HALOVISOR,
            Self::Automata => AUTOMATA_OPERATOR_HOLESKY,
            Self::AvaProtocol => AVA_OPERATOR,
            Self::ChainbaseNetwork => CHAINBASE_NETWORK_V2_NODE,
            Self::LagrangeStateCommittee => LAGRANGE_STATE_COMMITTEE_CONTAINER_NAME,
            Self::LagrangeZkWorker => LAGRANGE_WORKER_CONTAINER_NAME,
            Self::Nuffle => NUFFLE_CONTAINER_NAME,
            Self::PrimevMevCommit => PRIMEV_MEV_COMMIT_CONTAINER_NAME,
            Self::LagrangeZKProver => {
                return Err(NodeTypeError::SpecializedError(
                    "TODO".to_string(),
                ))
            }
            Self::Hyperlane => HYPERLANE_AGENT_CONTAINER_NAME,
            Self::WitnessChain => WITNESSCHAIN_CONTAINER_NAME,
            Self::GoPlusAVS => GOPLUS_CONTAINER_NAME,
            Self::UngateInfiniRoute(_infini_route_type) => UNGATE_NAME_1,
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
            Self::Brevis => return Err(NodeTypeError::NoDefaultContainerName),
            Self::K3LabsAvs => return Err(NodeTypeError::NoDefaultContainerName),
            Self::K3LabsAvsHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::AlignedLayer => return Err(NodeTypeError::InvalidNodeType),
            Self::SkateChain(_skate_chain_type) => return Err(NodeTypeError::NoDefaultContainerName),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::ArpaNetworkNodeClient => return Err(NodeTypeError::NoDefaultContainerName),
            Self::Predicate => return Err(NodeTypeError::NoDefaultContainerName),
            Self::AethosHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::OpenLayerHolesky => return Err(NodeTypeError::NoDefaultContainerName),
            Self::OpenLayerMainnet => return Err(NodeTypeError::InvalidNodeType),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
            Self::ChainbaseNetworkV1 => {
                return Err(NodeTypeError::SpecializedError(
                    "ChainbaseNetworkV1 is deprecated - update to V2 - ChainbaseNetwork"
                        .to_string(),
                ))
            }
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
            NodeType::LagrangeZKProver,
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
            NodeType::Gasp,
            NodeType::DittoNetwork,
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
            return Self::from_repo(parts[0]);
        }
        Self::from_repo(parts[1])
    }

    // Given a repo and tag, get the NodeType, since they have a 1:1 relationship
    pub fn from_repo(repo: &str) -> Option<Self> {
        debug!("repo: {}", repo);
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
            BREVIS_REPO => Some(Self::Brevis),
            GASP_REPO => Some(Self::Gasp),
            DITTO_NETWORK_REPO => Some(Self::DittoNetwork),
            NUFFLE_REPO => Some(Self::Nuffle),
            PRIMEV_LOCAL_REPO => Some(Self::PrimevMevCommit),
            PRIMEV_IMAGE_REPO => Some(Self::PrimevMevCommit),
            GOPLUS_REPO => Some(Self::GoPlusAVS),
            OMNI_REPO => Some(Self::Omni),
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
            CHAINBASE_NETWORK_V1_NODE => Self::ChainbaseNetwork,
            GOPLUS_CONTAINER_NAME => Self::GoPlusAVS,
            LAGRANGE_WORKER_CONTAINER_NAME => Self::LagrangeZkWorker,
            LAGRANGE_STATE_COMMITTEE_CONTAINER_NAME => Self::LagrangeStateCommittee,
            HYPERLANE_AGENT_CONTAINER_NAME => Self::Hyperlane,
            UNGATE_MAINNET => Self::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            UNGATE_NAME_1 => Self::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            UNGATE_NAME_2 => Self::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            UNGATE_NAME_3 => Self::UngateInfiniRoute(InfiniRouteType::UnknownL2),
            GASP_CONTAINER_NAME => Self::Gasp,
            DITTO_NETWORK_CONTAINER_NAME => Self::DittoNetwork,
            NUFFLE_CONTAINER_NAME => Self::Nuffle,
            NUFFLE_CONTAINER_NAME_2 => Self::Nuffle,
            PRIMEV_MEV_COMMIT_CONTAINER_NAME => Self::PrimevMevCommit,
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
