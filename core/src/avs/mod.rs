use crate::eigen::quorum::QuorumType;

use std::collections::HashMap;

use ethers::types::{Chain, U256};

pub mod config;
pub mod contracts;
pub mod eigenda;
pub mod lagrange;

pub type QuorumMinMap = HashMap<Chain, HashMap<QuorumType, U256>>;
