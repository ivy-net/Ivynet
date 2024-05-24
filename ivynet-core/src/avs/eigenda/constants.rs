use ethers::types::{Chain, U256};
use once_cell::sync::Lazy;
use std::collections::HashMap;

use super::eigenda::EigenDA;
use crate::{
    avs::{AvsConstants, QuorumMinMap},
    eigen::quorum::QuorumType,
};

impl AvsConstants for EigenDA {
    const QUORUM_CANDIDATES: Lazy<Vec<QuorumType>> = Lazy::new(|| vec![QuorumType::LST]);
    const QUORUM_MINS: Lazy<QuorumMinMap> = Lazy::new(|| {
        let mut m = HashMap::new();
        {
            // Mainnet
            let mut mainnet_map = HashMap::new();
            mainnet_map.insert(QuorumType::LST, U256::from(96 * (10 ^ 18)));
            m.insert(Chain::Mainnet, mainnet_map);
        }
        {
            // Holesky
            let mut holesky_map = HashMap::new();
            holesky_map.insert(QuorumType::LST, U256::from(96 * (10 ^ 18)));
            m.insert(Chain::Mainnet, holesky_map);
        }
        {
            // Local: TODO
        }
        m
    });
}
