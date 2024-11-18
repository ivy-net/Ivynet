use crate::IvyProvider;
use ethers::contract::abigen;

pub type LagrangeStakeRegistry = LagrangeStakeRegistryAbi<IvyProvider>;

abigen!(
    LagrangeStakeRegistryAbi,
    "abi/lagrange/ZKMRStakeRegistry.json",
    event_derives(serde::Deserialize, serde::Serialize)
);
