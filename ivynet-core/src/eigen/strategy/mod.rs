use ethers_core::types::Address;
use std::hash::Hash;
use std::{error::Error, fmt::Display};

pub mod holesky;
pub mod mainnet;

pub trait EigenStrategy: TryFrom<&'static str> + Eq + PartialEq + Hash + Copy + Clone {
    fn address(&self) -> Address;
}

pub trait StrategyList<T: EigenStrategy> {
    fn get_all() -> Vec<T>;
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
