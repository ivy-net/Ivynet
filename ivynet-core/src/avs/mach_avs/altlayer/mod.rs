use ethers_core::types::U256;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

use crate::avs::AvsVariant;
use crate::config;
use crate::eigen::node_classes::{self, NodeClass};
use crate::eigen::quorum::QuorumType;
use crate::rpc_management::Network;

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

    fn validate_node_size(&self, _: U256, _: u32) -> Result<bool, Box<dyn std::error::Error>> {
        let (_, _, disk_info) = config::get_system_information()?;
        let class = node_classes::get_node_class()?;
        // XL node + 50gb disk space
        Ok(class >= NodeClass::XL && disk_info >= 50000000000)
    }

    /// Currently, AltLayer Mach AVS is operating in allowlist mode only: https://docs.altlayer.io/altlayer-documentation/altlayer-facilitated-actively-validated-services/xterio-mach-avs-for-xterio-chain/operator-guide
    async fn optin(
        &self,
        quorums: Vec<crate::eigen::quorum::QuorumType>,
        network: crate::rpc_management::Network,
        eigen_path: std::path::PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    /// Quorum stake requirements can be found in the AltLayer docs: https://docs.altlayer.io/altlayer-documentation/altlayer-facilitated-actively-validated-services/xterio-mach-avs-for-xterio-chain/operator-guide
    fn quorum_min(
        &self,
        network: crate::rpc_management::Network,
        quorum_type: crate::eigen::quorum::QuorumType,
    ) -> U256 {
        match network {
            Network::Mainnet => match quorum_type {
                QuorumType::LST => U256::zero(),
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            Network::Holesky => match quorum_type {
                QuorumType::LST => U256::from(10 ^ 18), // one ETH
                QuorumType::EIGEN => todo!("Unimplemented"),
            },
            Network::Local => todo!("Unimplemented"),
        }
    }

    // TODO: Add Eigen quorum.
    fn quorum_candidates(&self, network: crate::rpc_management::Network) -> Vec<crate::eigen::quorum::QuorumType> {
        match network {
            Network::Mainnet => vec![QuorumType::LST],
            Network::Holesky => vec![QuorumType::LST],
            Network::Local => todo!("Unimplemented"),
        }
    }

    /// AltLayer stake registry contracts: https://github.com/alt-research/mach-avs
    fn stake_registry(&self, network: crate::rpc_management::Network) -> ethers_core::types::Address {
        match network {
            Network::Mainnet => "0x49296A7D4a76888370CB377CD909Cc73a2f71289".parse().unwrap(),
            Network::Holesky => "0x0b3eE1aDc2944DCcBb817f7d77915C7d38F7B858".parse().unwrap(),
            Network::Local => todo!("Unimplemented"),
        }
    }

    /// AltLayer registry coordinator contracts: https://github.com/alt-research/mach-avs
    fn registry_coordinator(&self, network: crate::rpc_management::Network) -> ethers_core::types::Address {
        match network {
            Network::Mainnet => "0x561be1AB42170a19f31645F774e6e3862B2139AA".parse().unwrap(),
            Network::Holesky => "0x1eA7D160d325B289bF981e0D7aB6Bf3261a0FFf2".parse().unwrap(),
            Network::Local => todo!("Unimplemented"),
        }
    }
}

pub async fn download_avs(setup_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
