use once_cell::sync::Lazy;
use std::collections::HashMap;

use super::eigenda::EigenDa;
use crate::{
    avs::{quorum::Quorum, AvsConstants},
    rpc_management::Network,
};

impl AvsConstants for EigenDa {
    fn quorums() -> Lazy<HashMap<Network, Vec<Quorum>>> {
        Lazy::new(|| {
            let mut m = HashMap::new();

            // TODO: add quorum 1
            let quorums =
                m.insert(Network::Mainnet, vec![Quorum::try_from_id_and_network(0, Network::Mainnet).unwrap()]);
            m
        })
    }
}
