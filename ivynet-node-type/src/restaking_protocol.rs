use super::{ActiveSet, AltlayerType, InfiniRouteType, MachType, NodeType, SkateChainType};

pub enum RestakingProtocolType {
    Eigenlayer,
    Symbiotic,
}

pub trait RestakingProtocol {
    fn restaking_protocol(&self) -> Option<RestakingProtocolType>;
}

impl RestakingProtocol for NodeType {
    fn restaking_protocol(&self) -> Option<RestakingProtocolType> {
        let protocol = match self {
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
            NodeType::WitnessChain => RestakingProtocolType::Eigenlayer,
            NodeType::Omni => RestakingProtocolType::Eigenlayer,
            NodeType::Automata => RestakingProtocolType::Eigenlayer,
            NodeType::OpenLayerMainnet => RestakingProtocolType::Eigenlayer,
            NodeType::OpenLayerHolesky => RestakingProtocolType::Eigenlayer,
            NodeType::AethosHolesky => RestakingProtocolType::Eigenlayer,
            NodeType::ArpaNetworkNodeClient => RestakingProtocolType::Eigenlayer,
            NodeType::UnifiAVS => RestakingProtocolType::Eigenlayer,
            NodeType::ChainbaseNetworkV1 => RestakingProtocolType::Eigenlayer,
            NodeType::ChainbaseNetwork => RestakingProtocolType::Eigenlayer,
            NodeType::DittoNetwork => RestakingProtocolType::Eigenlayer,
            NodeType::Primus => RestakingProtocolType::Eigenlayer,
            NodeType::GoPlusAVS => RestakingProtocolType::Eigenlayer,
            NodeType::AlignedLayer => RestakingProtocolType::Eigenlayer,
            NodeType::Brevis => RestakingProtocolType::Eigenlayer,
            NodeType::Nuffle => RestakingProtocolType::Eigenlayer,
            NodeType::Blockless => RestakingProtocolType::Eigenlayer,
            NodeType::AtlasNetwork => RestakingProtocolType::Eigenlayer,
            NodeType::Zellular => RestakingProtocolType::Eigenlayer,
            NodeType::Redstone => RestakingProtocolType::Eigenlayer,
            NodeType::MishtiNetwork => RestakingProtocolType::Eigenlayer,
            //Symbiotic
            NodeType::Cycle => RestakingProtocolType::Symbiotic,
            NodeType::Tanssi => RestakingProtocolType::Symbiotic,
            //Complicated
            NodeType::Altlayer(inner) => match inner {
                AltlayerType::Unknown => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },
            NodeType::AltlayerMach(inner) => match inner {
                MachType::Unknown => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },

            NodeType::SkateChain(inner) => match inner {
                SkateChainType::UnknownL2 => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },
            NodeType::UngateInfiniRoute(inner) => match inner {
                InfiniRouteType::UnknownL2 => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },

            NodeType::PrimevMevCommit(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            NodeType::Bolt(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            NodeType::Hyperlane(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
        };

        Some(protocol)
    }
}
