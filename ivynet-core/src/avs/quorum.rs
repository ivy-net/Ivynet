use ethers_core::types::Address;

use crate::{
    eigen::strategy::{holesky::HoleskyLstStrategies, mainnet::MainnetLstStrategies, EigenStrategy, StrategyList},
    rpc_management::Network,
};
use std::{error::Error, fmt::Display};

// TODO: For now these strategies are in the same enum across networks, but as the number of
// available quorums grow, this will likely become one enum per network.
pub enum Quorum {
    MainnetLst(Vec<MainnetLstStrategies>),
    HoleskyLst(Vec<HoleskyLstStrategies>),
}

impl Quorum {
    pub fn try_from_id_and_network(id: isize, network: Network) -> Result<Self, QuorumError> {
        let res = match network {
            Network::Mainnet => {
                let strats = match id {
                    0 => MainnetLstStrategies::get_all(),
                    1 => todo!("Eigen quorum unimplemented"), // eigen
                    _ => return Err(QuorumError::QuorumNotFound),
                };
                Quorum::MainnetLst(strats)
            }
            Network::Holesky => {
                let strats = match id {
                    0 => HoleskyLstStrategies::get_all(),
                    1 => todo!("Eigen quorum unimplemented"), // eigen
                    _ => return Err(QuorumError::QuorumNotFound),
                };
                Quorum::HoleskyLst(strats)
            }
            Network::Local => todo!(),
        };
        Ok(res)
    }

    pub fn stake_registry_id(&self) -> isize {
        match self {
            Quorum::MainnetLst(_) => 0,
            Quorum::HoleskyLst(_) => 0,
        }
    }
}

impl Into<Vec<Address>> for Quorum {
    fn into(self) -> Vec<Address> {
        match self {
            Quorum::HoleskyLst(inner) => inner.iter().map(|x| x.address()).collect(),
            Quorum::MainnetLst(inner) => inner.iter().map(|x| x.address()).collect(),
        }
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
