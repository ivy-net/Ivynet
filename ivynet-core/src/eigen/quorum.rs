use crate::rpc_management::Network;
use std::{error::Error, fmt::Display, ops::Deref};

use super::strategy::{holesky::HOLESKY_LST_STRATEGIES, mainnet::MAINNET_LST_STRATEGIES, Strategy};

#[derive(Clone, Debug)]
pub struct Quorum(pub Vec<Strategy>);

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
    pub fn try_from_type_and_network(quorum: QuorumType, network: Network) -> Result<Self, QuorumError> {
        let res = match network {
            Network::Mainnet => {
                let strats = match quorum as usize {
                    0 => MAINNET_LST_STRATEGIES.clone(),
                    1 => todo!("Eigen quorum unimplemented"), // eigen
                    _ => return Err(QuorumError::QuorumNotFound),
                };
                Quorum(strats.to_vec())
            }
            Network::Holesky => {
                let strats = match quorum as usize {
                    0 => HOLESKY_LST_STRATEGIES.clone(),
                    1 => todo!("Eigen quorum unimplemented"), // eigen
                    _ => return Err(QuorumError::QuorumNotFound),
                };
                Quorum(strats.to_vec())
            }
            Network::Local => todo!(),
        };
        Ok(res)
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
