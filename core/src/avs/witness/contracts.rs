use ethers::contract::abigen;

use crate::rpc_management::IvyProvider;

pub type OperatorRegistry = OperatorRegistryAbi<IvyProvider>;
pub type AvsDirectory = AvsDirectoryAbi<IvyProvider>;
pub type WitnessHub = WitnessHubAbi<IvyProvider>;

abigen!(
    OperatorRegistryAbi,
    "abi/witness/OperatorRegistry.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    AvsDirectoryAbi,
    "abi/witness/AvsDirectory.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    WitnessHubAbi,
    "abi/witness/WitnessHub.json",
    event_derives(serde::Deserialize, serde::Serialize)
);
