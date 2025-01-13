use ivynet_node_type::{NodeType, NodeTypeError};

use crate::RegistryType::{self, Chainbase, DockerHub, Github, GoogleCloud, Othentic, AWS};

pub trait ImageRegistry {
    fn registry(&self) -> Result<RegistryType, NodeTypeError>;
}

impl ImageRegistry for NodeType {
    fn registry(&self) -> Result<RegistryType, NodeTypeError> {
        let res = match self {
            Self::Gasp => DockerHub,
            Self::DittoNetwork => DockerHub,
            Self::EigenDA => Github,
            Self::EOracle => DockerHub,
            Self::AvaProtocol => DockerHub,
            Self::LagrangeStateCommittee => DockerHub,
            Self::LagrangeZkWorker => DockerHub,
            Self::LagrangeZKProver => DockerHub,
            Self::K3LabsAvs => DockerHub,
            Self::K3LabsAvsHolesky => DockerHub,
            Self::Predicate => Github,
            Self::Hyperlane => GoogleCloud,
            Self::WitnessChain => DockerHub,
            Self::Altlayer(_altlayer_type) => AWS,
            Self::AltlayerMach(_altlayer_mach_type) => AWS,
            Self::Omni => DockerHub,
            Self::Automata => Github,
            Self::OpenLayerMainnet => GoogleCloud,
            Self::OpenLayerHolesky => GoogleCloud,
            Self::AethosHolesky => Github,
            Self::ArpaNetworkNodeClient => Github,
            Self::ChainbaseNetworkV1 => Chainbase,
            Self::ChainbaseNetwork => Chainbase,
            Self::UngateInfiniRoute(_any) => Othentic,
            Self::GoPlusAVS => Othentic,
            Self::SkateChain(_any) => Othentic,
            Self::Brevis => return Err(NodeTypeError::NoRegistry),
            Self::Nuffle => return Err(NodeTypeError::NoRegistry),
            Self::AlignedLayer => return Err(NodeTypeError::NoRegistry),
            Self::PrimevMevCommit => return Err(NodeTypeError::NoRegistry),
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }
}
