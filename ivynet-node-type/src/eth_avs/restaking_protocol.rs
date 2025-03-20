use serde::{Deserialize, Serialize};

use super::{ActiveSet, AltlayerType, EthereumAvsType, InfiniRouteType, MachType, SkateChainType};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RestakingProtocolType {
    Eigenlayer,
    Symbiotic,
}

#[allow(dead_code)]
pub trait RestakingProtocol {
    fn restaking_protocol(&self) -> Option<RestakingProtocolType>;
}

impl RestakingProtocol for EthereumAvsType {
    fn restaking_protocol(&self) -> Option<RestakingProtocolType> {
        let protocol = match self {
            //Eigenlayer
            EthereumAvsType::Unknown => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::AvaProtocol => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::EigenDA => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::LagrangeStateCommittee => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::LagrangeZkWorker => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::LagrangeZKProver => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::K3LabsAvs => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::K3LabsAvsHolesky => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::EOracle => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Gasp => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Predicate => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::WitnessChain => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Omni => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Automata => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::OpenLayerMainnet => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::OpenLayerHolesky => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::AethosHolesky => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::ArpaNetworkNodeClient => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::UnifiAVS => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::ChainbaseNetworkV1 => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::ChainbaseNetwork => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Primus => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::GoPlusAVS => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::AlignedLayer => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Brevis => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Nuffle => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Blockless => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::AtlasNetwork => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Zellular => RestakingProtocolType::Eigenlayer,
            EthereumAvsType::Redstone => RestakingProtocolType::Eigenlayer,
            //Symbiotic
            EthereumAvsType::Cycle => RestakingProtocolType::Symbiotic,
            EthereumAvsType::Tanssi => RestakingProtocolType::Symbiotic,
            EthereumAvsType::PrimevBidder => RestakingProtocolType::Symbiotic,
            EthereumAvsType::Kalypso => RestakingProtocolType::Symbiotic,
            EthereumAvsType::RouterXtendNetwork => RestakingProtocolType::Symbiotic,
            EthereumAvsType::CapxCloud => RestakingProtocolType::Symbiotic,
            EthereumAvsType::Symbiosis => RestakingProtocolType::Symbiotic,
            EthereumAvsType::Radius => RestakingProtocolType::Symbiotic,
            EthereumAvsType::IBTCNetwork => RestakingProtocolType::Symbiotic,
            EthereumAvsType::ZKLink => RestakingProtocolType::Symbiotic,
            EthereumAvsType::HyveDA => RestakingProtocolType::Symbiotic,
            EthereumAvsType::BlessB7s => RestakingProtocolType::Symbiotic,
            //Complicated
            EthereumAvsType::DittoNetwork(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            EthereumAvsType::MishtiNetwork(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            EthereumAvsType::Altlayer(inner) => match inner {
                AltlayerType::Unknown => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },
            EthereumAvsType::AltlayerMach(inner) => match inner {
                MachType::Unknown => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },

            EthereumAvsType::SkateChain(inner) => match inner {
                SkateChainType::UnknownL2 => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },
            EthereumAvsType::UngateInfiniRoute(inner) => match inner {
                InfiniRouteType::UnknownL2 => return None,
                _ => RestakingProtocolType::Eigenlayer,
            },

            EthereumAvsType::PrimevMevCommit(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            EthereumAvsType::Bolt(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
            EthereumAvsType::Hyperlane(inner) => match inner {
                ActiveSet::Unknown => return None,
                ActiveSet::Eigenlayer => RestakingProtocolType::Eigenlayer,
                ActiveSet::Symbiotic => RestakingProtocolType::Symbiotic,
            },
        };

        Some(protocol)
    }
}
