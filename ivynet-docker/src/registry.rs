use ivynet_node_type::{NodeType, NodeTypeError};

use crate::RegistryType::{self, Chainbase, DockerHub, Github, GoogleCloud, Othentic, AWS};

pub trait ImageRegistry {
    fn registry(&self) -> Result<RegistryType, NodeTypeError>;
}

impl ImageRegistry for NodeType {
    fn registry(&self) -> Result<RegistryType, NodeTypeError> {
        let res = match self {
            Self::EigenDA => Github,
            Self::EOracle => DockerHub,
            Self::AvaProtocol => DockerHub,
            Self::LagrangeStateCommittee => DockerHub,
            Self::LagrangeZkWorkerMainnet => DockerHub,
            Self::LagrangeZkWorkerHolesky => DockerHub,
            Self::K3LabsAvs => DockerHub,
            Self::K3LabsAvsHolesky => DockerHub,
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
            Self::ArpaNetworkNodeClient => Github,
            Self::ChainbaseNetworkV1 => Chainbase,
            Self::ChainbaseNetwork => Chainbase,
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
}
