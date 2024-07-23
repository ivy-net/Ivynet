use ethers::contract::abigen;

use crate::rpc_management::IvyProvider;

pub type LagrangeStakeRegistry = LagrangeStakeRegistryAbi<IvyProvider>;

abigen!(
    LagrangeStakeRegistryAbi,
    "abi/lagrange/ZKMRStakeRegistry.json",
    event_derives(serde::Deserialize, serde::Serialize)
);
