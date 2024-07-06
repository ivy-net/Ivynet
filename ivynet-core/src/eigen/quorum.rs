use ethers::types::{Address, Chain};
use std::{error::Error, fmt::Display, ops::Deref};

use super::strategy::{holesky::HOLESKY_LST_STRATEGIES, mainnet::MAINNET_LST_STRATEGIES, Strategy};

/// A Quorum represents a set of strategies (ERC20 tokens) that are used to determine the operator's stake.
#[derive(Clone, Debug)]
pub struct Quorum(pub Vec<Strategy>);

impl Deref for Quorum {
    type Target = Vec<Strategy>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Currently, quorum types are the same across mainnet and testnet. If those diverge, this will
// need to change.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum QuorumType {
    LST = 0,
    EIGEN = 1,
}

impl Display for QuorumType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuorumType::LST => write!(f, "LST"),
            QuorumType::EIGEN => write!(f, "EIGEN"),
        }
    }
}

// TODO: Fix these clones
impl Quorum {
    pub fn try_from_type_and_network(quorum: QuorumType, chain: Chain) -> Result<Self, QuorumError> {
        let res = match chain {
            Chain::Mainnet => {
                let strats = match quorum as usize {
                    0 => MAINNET_LST_STRATEGIES.clone(),
                    1 => todo!("Eigen quorum unimplemented"), // eigen
                    _ => return Err(QuorumError::QuorumNotFound),
                };
                Quorum(strats)
            }
            Chain::Holesky => {
                let strats = match quorum as usize {
                    0 => HOLESKY_LST_STRATEGIES.clone(),
                    1 => todo!("Eigen quorum unimplemented"), // eigen
                    _ => return Err(QuorumError::QuorumNotFound),
                };
                Quorum(strats)
            }
            _ => todo!(),
        };
        Ok(res)
    }

    pub fn to_strategies(self) -> Vec<Strategy> {
        self.0
    }

    /// Converts the quorum to a vec of addresses of each strategy.
    pub fn to_addresses(self) -> Vec<Address> {
        self.0.iter().map(|strat| strat.address).collect()
    }
}

#[derive(Debug)]
pub enum QuorumError {
    QuorumNotFound,
}

impl Error for QuorumError {}

impl Display for QuorumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuorumError::QuorumNotFound => write!(f, "Quorum ID not found or registered"),
        }
    }
}
