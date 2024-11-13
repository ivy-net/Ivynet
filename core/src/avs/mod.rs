use std::collections::HashMap;

use crate::{config::IvyConfig, eigen::quorum::QuorumType, error::IvyError};

use dialoguer::Input;
use ethers::types::{Chain, U256};
use url::Url;

pub mod commands;
pub mod config;
pub mod contracts;
pub mod eigenda;
pub mod lagrange;
pub mod mach_avs;
pub mod names;

pub type QuorumMinMap = HashMap<Chain, HashMap<QuorumType, U256>>;

pub async fn fetch_rpc_url(chain: Chain, config: &IvyConfig) -> Result<Url, IvyError> {
    Ok(Input::<Url>::new()
        .with_prompt(format!("Enter your RPC URL for {chain:?}"))
        .default(config.get_default_rpc_url(chain)?.parse::<Url>()?)
        .interact_text()?)
}
