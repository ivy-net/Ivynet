use ethers_core::types::Address;
use std::hash::Hash;
use std::{error::Error, fmt::Display};

pub mod holesky;
pub mod mainnet;

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

pub trait EigenStrategy: TryFrom<&'static str> + Eq + PartialEq + Hash + Copy + Clone {
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
