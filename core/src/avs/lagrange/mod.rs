/// ZK Coprocessor AVS handler.
/// Because the Lagrange ZK Coprocessor doesn't have a simple way to configure the network the
/// AVS is running on (Requires a combination of environment variables, and editing the
/// docker-compose file directly), this module handles the Lagrage directory somewhat
/// differently, effectively duplicating it per-network. E.G.
/// `~/.eigenlayer/lagrange/holesky/lagrange-worker` and `~/.eigenlayer/lagrange/mainnet/
/// lagrange-worker`.
use ethers::types::{Chain, H160, U256};
use std::{
    fs::{self},
    path::PathBuf,
    process::Command,
    sync::Arc,
};
use thiserror::Error as ThisError;
use tracing::{debug, error};
use url::Url;

use crate::{
    dialog::get_confirm_password,
    eigen::quorum::QuorumType,
    env_parser::EnvLines,
    error::{IvyError, SetupError},
    rpc_management::IvyProvider,
};

use super::{
    config::{NodeConfig, NodeType},
    names::AvsName,
};

pub mod config;
pub mod setup;

/**
 *
 *   General process for setting up the Lagrange AVS:
 *   Create a lagrange key (No ecdsa dependencies)
 *   Copy the operator ecdsa key to the lagrange-worker/config path (priv_key.json)
 *   Register the lagrange key + priv_key
 *   Remove priv_key
 *   Start the docker container
 *
 */

#[derive(ThisError, Debug)]
pub enum LagrangeError {
    #[error("Boot script failed: {0}")]
    ScriptError(String),
    #[error("Failed to download resource: {0}")]
    DownloadFailedError(String),
    #[error("Keyfile password not found")]
    KeyfilePasswordNotFound,
}

// #[derive(Debug, Clone)]
// pub struct Lagrange {
//     #[allow(dead_code)]
//     base_path: PathBuf,
//     #[allow(dead_code)]
//     chain: Chain,
//     #[allow(dead_code)]
//     avs_config: NodeConfig,
// }
//
// impl Lagrange {
//     pub fn new(base_path: PathBuf, chain: Chain, avs_config: NodeConfig) -> Self {
//         Self { base_path, chain, avs_config }
//     }
// }
//
// impl Default for Lagrange {
//     fn default() -> Self {
//         todo!()
//         // let avs_config = AvsConfig::load(AvsName::LagrangeZK.as_str())
//         //     .expect("Could not load AVS config - go through setup");
//         // Self::new(avs_config.get_path(Chain::Holesky), Chain::Holesky, avs_config)
//     }
// }
//
// impl Lagrange {
//     async fn register(
//         &self,
//         _provider: Arc<IvyProvider>,
//         _eigen_path: PathBuf,
//         private_keypath: PathBuf,
//         keyfile_password: &str,
//     ) -> Result<(), IvyError> {
//         // Copy keyfile to current dir
//         let dest_dir = self.run_path().join("config");
//         if !dest_dir.exists() {
//             fs::create_dir_all(dest_dir.clone())?;
//         }
//         let dest_file = dest_dir.join("priv_key.json");
//
//         debug!("{}", dest_file.display());
//         fs::copy(private_keypath, &dest_file)?;
//         // Change dir to run docker file
//         std::env::set_current_dir(self.run_path())?;
//         // Set local env variable to pass password to docker
//         std::env::set_var("AVS__ETH_PWD", keyfile_password);
//         let _ = Command::new("docker")
//             .arg("compose")
//             .arg("run")
//             .args(["--rm", "worker", "avs", "register"])
//             .status()?;
//         fs::remove_file(dest_file)?;
//         Ok(())
//     }
// }
