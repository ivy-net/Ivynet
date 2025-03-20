use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct EthereumNode {
    execution: ExecutionClientType,
    consensus: Option<ConsensusClientType>,
    validator: Option<ValidatorType>,
    sidecars: Vec<SidecarType>,
}

/// Used for grabbing repos
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum EthereumComponentType {
    Consensus(ConsensusClientType),
    Execution(ExecutionClientType),
    Validator(ValidatorType),
    Sidecar(SidecarType),
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, Default)]
pub enum ValidatorType {
    #[default]
    Unknown,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, Default)]
pub enum SidecarType {
    #[default]
    Unknown,
    MEVBoost,
    SSV,
}
