use ethers_core::types::U256;
use std::collections::HashMap;
use std::error::Error;

use super::AvsVariant;

#[derive(Default)]
pub struct MachAvs {}

impl AvsVariant for MachAvs {
    async fn setup(&self, env_path: std::path::PathBuf) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    async fn build_env(
        &self,
        env_path: std::path::PathBuf,
        network: crate::rpc_management::Network,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn validate_node_size(&self, quorum_percentage: U256, bandwidth: u32) -> Result<bool, Box<dyn std::error::Error>> {
        todo!()
    }

    async fn optin(
        &self,
        quorums: Vec<crate::eigen::quorum::QuorumType>,
        network: crate::rpc_management::Network,
        eigen_path: std::path::PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn quorum_min(
        &self,
        network: crate::rpc_management::Network,
        quorum_type: crate::eigen::quorum::QuorumType,
    ) -> U256 {
        todo!()
    }

    fn quorum_candidates(&self, network: crate::rpc_management::Network) -> Vec<crate::eigen::quorum::QuorumType> {
        todo!()
    }

    fn stake_registry(&self, network: crate::rpc_management::Network) -> ethers_core::types::Address {
        todo!()
    }

    fn registry_coordinator(&self, network: crate::rpc_management::Network) -> ethers_core::types::Address {
        todo!()
    }
}
