use self::{
    eigenda::{
        eigenda::build_quorums,
        eigenda_info::{self, RegistryCoordinator, RegistryCoordinatorSigner, StakeRegistry},
    },
    quorum::Quorum,
};
use crate::{
    config,
    eigen::{
        dgm_info::EigenStrategy,
        node_classes::{self, NodeClass},
    },
    rpc_management::Network,
};
use ethers_core::types::{Address, U256};
use once_cell::sync::Lazy;
use std::{collections::HashMap, path::PathBuf};

pub mod avs_default;
pub mod eigenda;
pub mod mach_avs;
pub mod quorum;

pub struct EigenDa {}

pub struct MachAvs {}

pub enum AvsType {
    EigenDa(EigenDa),
    MachAvs(MachAvs),
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

impl BootableAvs for EigenDa {
    async fn boot() {
        todo!()
    }

    async fn build_env_file(network: Network, eigen_path: PathBuf) {
        todo!()
    }

    fn edit_env(filename: &str, env_values: HashMap<&str, &str>) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    fn optin(quorums: String, network: Network, eigen_path: PathBuf) {
        todo!()
    }

    fn status(addr: Address) {
        todo!()
    }

    fn check_stake(addr: Address, network: Network) {
        todo!()
    }

    fn check_system_mins(quorum_percentage: U256, bandwidth: u32) -> Result<bool, Box<dyn std::error::Error>> {
        let (_, _, disk_info) = config::get_system_information()?;
        let class = node_classes::get_node_class()?;
        let mut acceptable = false;
        match quorum_percentage {
            x if x < U256::from(3) => {
                if class >= NodeClass::LRG || bandwidth >= 1 || disk_info >= 20000000000 {
                    acceptable = true
                }
            }
            x if x < U256::from(20) => {
                if class >= NodeClass::XL || bandwidth >= 1 || disk_info >= 150000000000 {
                    acceptable = true
                }
            }
            x if x < U256::from(100) => {
                if class >= NodeClass::FOURXL || bandwidth >= 3 || disk_info >= 750000000000 {
                    acceptable = true
                }
            }
            x if x < U256::from(1000) => {
                if class >= NodeClass::FOURXL || bandwidth >= 25 || disk_info >= 4000000000000 {
                    acceptable = true
                }
            }
            x if x > U256::from(2000) => {
                if class >= NodeClass::FOURXL || bandwidth >= 50 || disk_info >= 8000000000000 {
                    acceptable = true
                }
            }
            _ => {}
        }
        Ok(acceptable)
    }
}

pub trait AvsConstants {
    fn quorums() -> Lazy<HashMap<Network, Vec<Quorum>>>;
}
