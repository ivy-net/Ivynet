use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

/// Used for grabbing repos
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EthereumNodeComponent {
    Consensus(ConsensusClient),
    Execution(ExecutionClient),
    Validator(Validator),
    Sidecar(Sidecar),
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, Default)]
pub enum ConsensusClient {
    Lighthouse,
    Prysm,
    Nimbus,
    Teku,
    Lodestar,
    #[default]
    Unknown,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, Default)]
pub enum ExecutionClient {
    Geth,
    Erigon,
    Nethermind,
    Besu,
    Reth,
    #[default]
    Unknown,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, Default)]
pub enum Validator {
    #[default]
    Unknown,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, Default)]
pub enum Sidecar {
    #[default]
    Unknown,
    MEVBoost,
    SSV,
}

impl IntoEnumIterator for EthereumNodeComponent {
    type Iterator = std::vec::IntoIter<EthereumNodeComponent>;

    fn iter() -> Self::Iterator {
        vec![]
            .into_iter()
            .chain(ConsensusClient::iter().map(EthereumNodeComponent::Consensus))
            .chain(ExecutionClient::iter().map(EthereumNodeComponent::Execution))
            .chain(Validator::iter().map(EthereumNodeComponent::Validator))
            .chain(Sidecar::iter().map(EthereumNodeComponent::Sidecar))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl EthereumNodeComponent {
    pub fn from_str(s: &str) -> Option<Self> {
        let normalized = s.replace(['-', '_', ' '], "").to_lowercase();

        //FIXME: Needs a lot of work when I figure out eth nodes from ethdocker, etc.

        // First try exact match (current behavior)
        let exact_match = EthereumNodeComponent::iter().find(|variant| {
            let variant_str = format!("{:?}", variant);
            let variant_normalized = variant_str.replace(['-', '_', ' '], "").to_lowercase();
            normalized == variant_normalized
        });

        if let Some(exact_match) = exact_match {
            return Some(exact_match);
        }

        // If no exact match, try matching just the outer type
        match normalized.as_str() {
            "consensus" => Some(Self::Consensus(ConsensusClient::Unknown)),
            "execution" => Some(Self::Execution(ExecutionClient::Unknown)),
            "validator" => Some(Self::Validator(Validator::Unknown)),
            "sidecar" => Some(Self::Sidecar(Sidecar::Unknown)),
            _ => None,
        }
    }
}
