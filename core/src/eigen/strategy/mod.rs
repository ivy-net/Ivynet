use std::{error::Error, fmt::Display, hash::Hash};

use ethers::types::{Address, Chain};

pub mod holesky;
pub mod mainnet;

// TODO: As a strategy is an ERC20 token, this may be implemented as an alias to an ERC20 type
#[derive(Clone, Debug)]
pub struct Strategy {
    pub name: String,
    pub address: Address,
}

impl Strategy {
    fn new(name: &str, address: Address) -> Self {
        Self { name: name.to_owned(), address }
    }
}

pub trait StrategyList: TryFrom<&'static str> + Eq + PartialEq + Hash + Copy + Clone {
    fn address(&self) -> Address;
}

#[derive(Debug)]
pub enum StrategyError {
    UnknownStrategy,
}

impl Display for StrategyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyError::UnknownStrategy => write!(f, "Unknown Strategy"),
        }
    }
}

impl Error for StrategyError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum EigenStrategy {
    Weth,
    BeaconEth,
    Reth,
    Oseth,
    Steth,
    Ankreth,
    Meth,
    Ethx,
    Lseth,
    Cbeth,
    Sfrxeth,
    Sweth,
    Oeth,
    Wbeth,
    Unknown,
}

pub fn get_strategy_list(chain: Chain) -> Vec<EigenStrategy> {
    match chain {
        Chain::Holesky => vec![
            EigenStrategy::Steth,
            EigenStrategy::Reth,
            EigenStrategy::Weth,
            EigenStrategy::Lseth,
            EigenStrategy::Sfrxeth,
            EigenStrategy::Ethx,
            EigenStrategy::Oseth,
            EigenStrategy::Cbeth,
            EigenStrategy::Meth,
            EigenStrategy::Ankreth,
            EigenStrategy::BeaconEth,
        ],
        Chain::Mainnet => vec![
            EigenStrategy::Cbeth,
            EigenStrategy::Steth,
            EigenStrategy::Reth,
            EigenStrategy::Sweth,
            EigenStrategy::Lseth,
            EigenStrategy::Sfrxeth,
            EigenStrategy::Wbeth,
            EigenStrategy::Ethx,
            EigenStrategy::Oseth,
            EigenStrategy::Meth,
            EigenStrategy::Ankreth,
            EigenStrategy::BeaconEth,
            EigenStrategy::Oeth,
        ],
        _ => todo!(),
    }
}
