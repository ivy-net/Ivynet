use ivynet_node_type::{NodeType, NodeTypeError};

use crate::RegistryType::{self, Chainbase, DockerHub, Github, GoogleCloud, Local, Othentic, AWS};

pub trait ImageRegistry {
    fn registry(&self) -> Result<RegistryType, NodeTypeError>;
}

impl ImageRegistry for NodeType {
    fn registry(&self) -> Result<RegistryType, NodeTypeError> {
        let res = match self {
            Self::Redstone => Othentic,
            Self::Bolt => Github,
            Self::Zellular => DockerHub,
            Self::AtlasNetwork => DockerHub,
            Self::Primus => DockerHub,
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
            Self::GoPlusAVS => Local,
            Self::SkateChain(_any) => Othentic,
            Self::Brevis => Local,
            Self::Nuffle => Local,
            Self::AlignedLayer => Local,
            Self::PrimevMevCommit => Local,
            Self::Blockless => Local,
            Self::UnifiAVS => return Err(NodeTypeError::InvalidNodeType),
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
    }
}
