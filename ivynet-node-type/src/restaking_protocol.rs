use super::NodeType;

pub enum RestakingProtocolType {
    Eigenlayer,
    Symbiotic,
}

pub trait RestakingProtocol {
    fn restaking_protocol(&self) -> RestakingProtocolType;
}

impl RestakingProtocol for NodeType {
    fn restaking_protocol(&self) -> RestakingProtocolType {
        match self {
            //Eigenlayer
            NodeType::Unknown => RestakingProtocolType::Eigenlayer,
            NodeType::AvaProtocol => RestakingProtocolType::Eigenlayer,
            NodeType::EigenDA => RestakingProtocolType::Eigenlayer,
            NodeType::LagrangeStateCommittee => RestakingProtocolType::Eigenlayer,
            NodeType::LagrangeZkWorker => RestakingProtocolType::Eigenlayer,
            NodeType::LagrangeZKProver => RestakingProtocolType::Eigenlayer,
            NodeType::K3LabsAvs => RestakingProtocolType::Eigenlayer,
            NodeType::K3LabsAvsHolesky => RestakingProtocolType::Eigenlayer,
            NodeType::EOracle => RestakingProtocolType::Eigenlayer,
            NodeType::Gasp => RestakingProtocolType::Eigenlayer,
            NodeType::Predicate => RestakingProtocolType::Eigenlayer,
            NodeType::Hyperlane => RestakingProtocolType::Eigenlayer,
            NodeType::WitnessChain => RestakingProtocolType::Eigenlayer,
            NodeType::Altlayer(_) => RestakingProtocolType::Eigenlayer,
            NodeType::AltlayerMach(_) => RestakingProtocolType::Eigenlayer,
            NodeType::Omni => RestakingProtocolType::Eigenlayer,
            NodeType::Automata => RestakingProtocolType::Eigenlayer,
            NodeType::OpenLayerMainnet => RestakingProtocolType::Eigenlayer,
            NodeType::OpenLayerHolesky => RestakingProtocolType::Eigenlayer,
            NodeType::AethosHolesky => RestakingProtocolType::Eigenlayer,
            NodeType::ArpaNetworkNodeClient => RestakingProtocolType::Eigenlayer,
            NodeType::UnifiAVS => RestakingProtocolType::Eigenlayer,
            NodeType::ChainbaseNetworkV1 => RestakingProtocolType::Eigenlayer,
            NodeType::SkateChain(_) => RestakingProtocolType::Eigenlayer,
            NodeType::ChainbaseNetwork => RestakingProtocolType::Eigenlayer,
            NodeType::DittoNetwork => RestakingProtocolType::Eigenlayer,
            NodeType::Primus => RestakingProtocolType::Eigenlayer,
            NodeType::GoPlusAVS => RestakingProtocolType::Eigenlayer,
            NodeType::UngateInfiniRoute(_) => RestakingProtocolType::Eigenlayer,
            NodeType::PrimevMevCommit => RestakingProtocolType::Eigenlayer,
            NodeType::AlignedLayer => RestakingProtocolType::Eigenlayer,
            NodeType::Brevis => RestakingProtocolType::Eigenlayer,
            NodeType::Nuffle => RestakingProtocolType::Eigenlayer,
            NodeType::Blockless => RestakingProtocolType::Eigenlayer,
            NodeType::AtlasNetwork => RestakingProtocolType::Eigenlayer,
            NodeType::Zellular => RestakingProtocolType::Eigenlayer,
            NodeType::Bolt => RestakingProtocolType::Eigenlayer,
            NodeType::Redstone => RestakingProtocolType::Eigenlayer,
            NodeType::MishtiNetwork => RestakingProtocolType::Eigenlayer,
            //Symbiotic
        }
    }
}
