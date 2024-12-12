use serde::{Deserialize, Serialize};

const EIGENDA_METRICS_ID: &str = "da-node";

pub const AVA_PROTOCOL: &str = "ava-protocol";
pub const EIGENDA: &str = "eigenda";
pub const LAGRANGE_STATE_COMMITTEE: &str = "lagrange-state-committee";
pub const LAGRANGE_ZK_WORKER_HOLESKY: &str = "lagrange-zk-worker-holesky";
pub const LAGRANGE_ZK_WORKER_MAINNET: &str = "lagrange-zk-worker-mainnet";
pub const K3_LABS_AVS: &str = "k3-labs-avs";
pub const EORACLE: &str = "eoracle";
pub const PREDICATE: &str = "predicate-operator";
pub const HYPERLANE: &str = "hyperlane";
pub const BREVIS: &str = "brevis";
pub const WITNESSCHAIN: &str = "witnesschain";
//New
pub const ALTLAYER_MACH: &str = "altlayer-mach";
pub const XTERIO_MACH: &str = "xterio-mach";
pub const OMNI: &str = "omni";
pub const AUTOMATA: &str = "automata";
pub const DODOCHAIN: &str = "dodochain";
pub const OPENLAYER: &str = "openlayer";
pub const CYBERMACH: &str = "cyber-mach";
pub const AETHOS: &str = "aethos";
pub const ARPANETWORK: &str = "arpa-network";
pub const OPACITYNETWORK: &str = "opacity-network";
pub const GMNETWORKMACH: &str = "gm-network-mach";
pub const UNIFIAVS: &str = "unifi-avs";
pub const SKATECHAINBASE: &str = "skate-chain-base";
pub const SKATECHAINMANTLE: &str = "skate-chain-mantle";
pub const CHAINBASENETWORKAVS: &str = "chainbase-network-avs";
pub const GOPLUSAVS: &str = "go-plus-avs";
pub const UNGATEINFINIROUTEBASE: &str = "ungate-infini-route-base";
pub const UNGATEINFINIROUTEPOLYGON: &str = "ungate-infini-route-polygon";
pub const PRIMEVMEVCOMMIT: &str = "primev-mev-commit";
pub const ALIGNEDLAYER: &str = "aligned-layer";

// const LAGRANGE_MAINNET_WORKER_IMAGE_NAME: &str = "lagrangelabs/worker:mainnet";

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            AVA_PROTOCOL => Self::AvaProtocol,
            EIGENDA => Self::EigenDA,
            LAGRANGE_STATE_COMMITTEE => Self::LagrangeStateCommittee,
            LAGRANGE_ZK_WORKER_HOLESKY => Self::LagrangeZkWorkerHolesky,
            LAGRANGE_ZK_WORKER_MAINNET => Self::LagrangeZkWorkerMainnet,
            K3_LABS_AVS => Self::K3LabsAvs,
            EORACLE => Self::EOracle,
            PREDICATE => Self::Predicate,
            HYPERLANE => Self::Hyperlane,
            BREVIS => Self::Brevis,
            WITNESSCHAIN => Self::WitnessChain,
            ALTLAYER_MACH => Self::AltlayerMach,
            XTERIO_MACH => Self::XterioMACH,
            OMNI => Self::Omni,
            AUTOMATA => Self::Automata,
            DODOCHAIN => Self::DodoChain,
            OPENLAYER => Self::OpenLayer,
            CYBERMACH => Self::CyberMach,
            AETHOS => Self::Aethos,
            ARPANETWORK => Self::ArpaNetwork,
            OPACITYNETWORK => Self::OpacityNetwork,
            GMNETWORKMACH => Self::GMNetworkMach,
            UNIFIAVS => Self::UnifiAVS,
            SKATECHAINBASE => Self::SkateChainBase,
            SKATECHAINMANTLE => Self::SkateChainMantle,
            CHAINBASENETWORKAVS => Self::ChainbaseNetworkAVS,
            GOPLUSAVS => Self::GoPlusAVS,
            UNGATEINFINIROUTEBASE => Self::UngateInfiniRouteBase,
            UNGATEINFINIROUTEPOLYGON => Self::UngateInfiniRoutePolygon,
            PRIMEVMEVCOMMIT => Self::PrimevMevCommit,
            ALIGNEDLAYER => Self::AlignedLayer,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AvaProtocol => write!(f, "{}", AVA_PROTOCOL),
            Self::EigenDA => write!(f, "{}", EIGENDA),
            Self::LagrangeStateCommittee => write!(f, "{}", LAGRANGE_STATE_COMMITTEE),
            Self::LagrangeZkWorkerHolesky => write!(f, "{}", LAGRANGE_ZK_WORKER_HOLESKY),
            Self::LagrangeZkWorkerMainnet => write!(f, "{}", LAGRANGE_ZK_WORKER_MAINNET),
            Self::K3LabsAvs => write!(f, "{}", K3_LABS_AVS),
            Self::EOracle => write!(f, "{}", EORACLE),
            Self::Predicate => write!(f, "{}", PREDICATE),
            Self::Hyperlane => write!(f, "{}", HYPERLANE),
            Self::Brevis => write!(f, "{}", BREVIS),
            Self::WitnessChain => write!(f, "{}", WITNESSCHAIN),
            Self::AltlayerMach => write!(f, "{}", ALTLAYER_MACH),
            Self::XterioMACH => write!(f, "{}", XTERIO_MACH),
            Self::Omni => write!(f, "{}", OMNI),
            Self::Automata => write!(f, "{}", AUTOMATA),
            Self::DodoChain => write!(f, "{}", DODOCHAIN),
            Self::OpenLayer => write!(f, "{}", OPENLAYER),
            Self::CyberMach => write!(f, "{}", CYBERMACH),
            Self::Aethos => write!(f, "{}", AETHOS),
            Self::ArpaNetwork => write!(f, "{}", ARPANETWORK),
            Self::OpacityNetwork => write!(f, "{}", OPACITYNETWORK),
            Self::GMNetworkMach => write!(f, "{}", GMNETWORKMACH),
            Self::UnifiAVS => write!(f, "{}", UNIFIAVS),
            Self::SkateChainBase => write!(f, "{}", SKATECHAINBASE),
            Self::SkateChainMantle => write!(f, "{}", SKATECHAINMANTLE),
            Self::ChainbaseNetworkAVS => write!(f, "{}", CHAINBASENETWORKAVS),
            Self::GoPlusAVS => write!(f, "{}", GOPLUSAVS),
            Self::UngateInfiniRouteBase => write!(f, "{}", UNGATEINFINIROUTEBASE),
            Self::UngateInfiniRoutePolygon => write!(f, "{}", UNGATEINFINIROUTEPOLYGON),
            Self::PrimevMevCommit => write!(f, "{}", PRIMEVMEVCOMMIT),
            Self::AlignedLayer => write!(f, "{}", ALIGNEDLAYER),
            Self::Unknown => write!(f, "unknown"),
        }
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

            Self::AvaProtocol => todo!(),
            Self::EigenDA => todo!(),
            Self::LagrangeStateCommittee => todo!(),
            Self::LagrangeZkWorkerHolesky => todo!(),
            Self::LagrangeZkWorkerMainnet => todo!(),
            Self::K3LabsAvs => todo!(),
            Self::EOracle => todo!(),
            Self::Predicate => todo!(),
            Self::Hyperlane => todo!(),
            Self::Brevis => todo!(),
            Self::WitnessChain => todo!(),
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
            Self::AvaProtocol => todo!(),
            Self::EigenDA => todo!(),
            Self::LagrangeStateCommittee => todo!(),
            Self::LagrangeZkWorkerHolesky => todo!(),
            Self::LagrangeZkWorkerMainnet => todo!(),
            Self::K3LabsAvs => todo!(),
            Self::EOracle => todo!(),
            Self::Predicate => todo!(),
            Self::Hyperlane => todo!(),
            Self::Brevis => todo!(),
            Self::WitnessChain => todo!(),
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
            Self::AvaProtocol => todo!(),
            Self::EigenDA => todo!(),
            Self::LagrangeStateCommittee => todo!(),
            Self::LagrangeZkWorkerHolesky => todo!(),
            Self::LagrangeZkWorkerMainnet => todo!(),
            Self::K3LabsAvs => todo!(),
            Self::EOracle => todo!(),
            Self::Predicate => todo!(),
            Self::Hyperlane => todo!(),
            Self::Brevis => todo!(),
            Self::WitnessChain => todo!(),
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
}
