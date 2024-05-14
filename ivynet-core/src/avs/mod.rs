use std::{collections::HashMap, path::PathBuf};

use crate::rpc_management::Network;

pub mod avs_default;
pub mod eigenda;

/// Trait for managing AVS instances.
///
/// Async traits still have some limitations. See `https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html for reference.`
pub trait Avs {
    async fn boot();
    async fn build_env_file(network: Network, eigen_path: PathBuf);
    fn edit_env_vars(filename: &str, env_values: HashMap<&str, &str>) -> Result<(), Box<dyn std::error::Error>>;
    fn optin(quorums: String, network: Network, eigen_path: PathBuf);
}
