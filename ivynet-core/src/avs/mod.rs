use crate::{eigen::quorum::QuorumType, rpc_management::Network};
use ethers_core::types::{Address, U256};
use once_cell::sync::Lazy;
use std::{collections::HashMap, path::PathBuf};

pub mod avs_default;
pub mod eigenda;
pub mod mach_avs;

pub type QuorumMinMap = HashMap<Network, HashMap<QuorumType, U256>>;

pub trait AvsConstants {
    const QUORUM_CANDIDATES: Lazy<Vec<QuorumType>>;
    const QUORUM_MINS: Lazy<QuorumMinMap>;
}

/// Trait for managing AVS instances.
///
/// Async traits still have some limitations. See `https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html for reference.`
pub trait BootableAvs {
    async fn boot();
    async fn build_env_file(network: Network, eigen_path: PathBuf);
    fn edit_env(filename: &str, env_values: HashMap<&str, &str>) -> Result<(), Box<dyn std::error::Error>>;
    fn optin(quorums: String, network: Network, eigen_path: PathBuf);
    fn status(addr: Address);
    fn check_stake(addr: Address, network: Network);
    fn check_system_mins(quorum_percentage: U256, bandwidth: u32) -> Result<bool, Box<dyn std::error::Error>>;
}
