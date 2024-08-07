use ethers::contract::abigen;

use crate::rpc_management::IvyProvider;

pub type StakeRegistry = StakeRegistryAbi<IvyProvider>;
pub type RegistryCoordinator = RegistryCoordinatorAbi<IvyProvider>;

pub mod lagrange;

abigen!(
    RegistryCoordinatorAbi,
    "abi/RegistryCoordinator.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    StakeRegistryAbi,
    "abi/StakeRegistry.json",
    event_derives(serde::Deserialize, serde::Serialize)
);
