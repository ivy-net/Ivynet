use ethers_core::types::U256;
use std::collections::HashMap;
use std::error::Error;

use crate::{
    config,
    eigen::{
        dgm_info::EigenStrategy,
        node_classes::{self, NodeClass},
    },
    rpc_management::Network,
};

pub type NodeClassBucket = HashMap<NodeClass, U256>;

pub struct MachAvs {
    quorum_node_classes: NodeClassBucket,
}

pub struct Requirements {
    bandwidth: u64,
    memory: u64,
}

impl MachAvs {
    fn validate_system_requirements() -> Result<bool, Box<dyn Error>> {
        let class = node_classes::get_node_class()?;
        let (_, _, disk_info) = config::get_system_information()?;
        let acceptable = class >= NodeClass::XL && disk_info < 50000000000;
        Ok(acceptable)
    }

    // fn validate_staking_requirements(network: Network) -> Result<bool, Box<dyn Error>> {
    //     let mut total_stake: u128 = 0;
    //     let required_stake = match network {
    //         Network::Mainnet => 0,
    //         Network::Holesky => 1 * (10 ^ 18),
    //         Network::Local => todo!(),
    //     };
    // }
}
