use serde::{Deserialize, Serialize};

use super::{ActiveSet, AltlayerType, EthereumAvs, InfiniRouteType, MachType, SkateChainType};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RestakingProtocolType {
    Eigenlayer,
    Symbiotic,
}

#[allow(dead_code)]
pub trait RestakingProtocol {
    fn restaking_protocol(&self) -> Option<RestakingProtocolType>;
}

impl RestakingProtocol for EthereumAvs {
    fn restaking_protocol(&self) -> Option<RestakingProtocolType> {
        let protocol = match self {
            //Eigenlayer
            EthereumAvs::AvaProtocol => RestakingProtocolType::Eigenlayer,
            EthereumAvs::EigenDA => RestakingProtocolType::Eigenlayer,
            EthereumAvs::LagrangeStateCommittee => RestakingProtocolType::Eigenlayer,
            EthereumAvs::LagrangeZkWorker => RestakingProtocolType::Eigenlayer,
            EthereumAvs::LagrangeZKProver => RestakingProtocolType::Eigenlayer,
            EthereumAvs::K3LabsAvs => RestakingProtocolType::Eigenlayer,
            EthereumAvs::K3LabsAvsHolesky => RestakingProtocolType::Eigenlayer,
            EthereumAvs::EOracle => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Gasp => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Predicate => RestakingProtocolType::Eigenlayer,
            EthereumAvs::WitnessChain => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Omni => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Automata => RestakingProtocolType::Eigenlayer,
            EthereumAvs::OpenLayerMainnet => RestakingProtocolType::Eigenlayer,
            EthereumAvs::OpenLayerHolesky => RestakingProtocolType::Eigenlayer,
            EthereumAvs::AethosHolesky => RestakingProtocolType::Eigenlayer,
            EthereumAvs::ArpaNetworkNodeClient => RestakingProtocolType::Eigenlayer,
            EthereumAvs::UnifiAVS => RestakingProtocolType::Eigenlayer,
            EthereumAvs::ChainbaseNetworkV1 => RestakingProtocolType::Eigenlayer,
            EthereumAvs::ChainbaseNetwork => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Primus => RestakingProtocolType::Eigenlayer,
            EthereumAvs::GoPlusAVS => RestakingProtocolType::Eigenlayer,
            EthereumAvs::AlignedLayer => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Brevis => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Nuffle => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Blockless => RestakingProtocolType::Eigenlayer,
            EthereumAvs::AtlasNetwork => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Zellular => RestakingProtocolType::Eigenlayer,
            EthereumAvs::Redstone => RestakingProtocolType::Eigenlayer,
            //Symbiotic
            EthereumAvs::Cycle => RestakingProtocolType::Symbiotic,
            EthereumAvs::Tanssi => RestakingProtocolType::Symbiotic,
            EthereumAvs::PrimevBidder => RestakingProtocolType::Symbiotic,
            EthereumAvs::Kalypso => RestakingProtocolType::Symbiotic,
            EthereumAvs::RouterXtendNetwork => RestakingProtocolType::Symbiotic,
            EthereumAvs::CapxCloud => RestakingProtocolType::Symbiotic,
            EthereumAvs::Symbiosis => RestakingProtocolType::Symbiotic,
            EthereumAvs::Radius => RestakingProtocolType::Symbiotic,
            EthereumAvs::IBTCNetwork => RestakingProtocolType::Symbiotic,
            EthereumAvs::ZKLink => RestakingProtocolType::Symbiotic,
            EthereumAvs::HyveDA => RestakingProtocolType::Symbiotic,
            EthereumAvs::BlessB7s => RestakingProtocolType::Symbiotic,
            //Complicated
            EthereumAvs::DittoNetwork(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            EthereumAvs::MishtiNetwork(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            EthereumAvs::Altlayer(inner) => match inner {
                AltlayerType::Unknown => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },
            EthereumAvs::AltlayerMach(inner) => match inner {
                MachType::Unknown => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },

            EthereumAvs::SkateChain(inner) => match inner {
                SkateChainType::UnknownL2 => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },
            EthereumAvs::UngateInfiniRoute(inner) => match inner {
                InfiniRouteType::UnknownL2 => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },

            EthereumAvs::PrimevMevCommit(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            EthereumAvs::Bolt(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            EthereumAvs::Hyperlane(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
        };

        Some(protocol)
    }
}
