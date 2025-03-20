use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use tracing::{debug, error, warn};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum EthereumNodeType {
    Consensus(Option<ConsensusClientType>),
    Execution(ExecutionClientType),
    Validator,
    MEVBoost,
    SSV,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, Default)]
pub enum ConsensusClientType {
    Lighthouse,
    Prysm,
    Nimbus,
    Teku,
    Lodestar,
    #[default]
    Unknown,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, Default)]
pub enum ExecutionClientType {
    Geth,
    Erigon,
    Nethermind,
    Besu,
    Reth,
    #[default]
    Unknown,
}
