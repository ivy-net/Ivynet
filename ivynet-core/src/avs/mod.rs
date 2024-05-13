use std::path::PathBuf;

use crate::rpc_management::Network;

pub mod avs_default;
pub mod eigenda;

/// Trait for managing AVS instances.
///
/// Async traits still have some limitations. See `https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html for reference.`
pub trait Avs {
    async fn boot();
    async fn build_env_file(network: Network, eigen_path: PathBuf);
    fn optin(quorums: String, network: Network, eigen_path: PathBuf);
}
