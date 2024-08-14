use crate::avs::witness::WitnessError;
use crate::rpc_management::IvyProvider;
use ethers::{
    contract::abigen,
    types::{Chain, H160},
};
use ivynet_macros::h160;

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

pub(super) fn operator_registry(chain: Chain) -> Result<H160, WitnessError> {
    match chain {
        Chain::Mainnet => Ok(h160!(0xef1a89841fd189ba28e780a977ca70eb1a5e985d)),
        Chain::Holesky => Ok(h160!(0x708CBDDdab358c1fa8efB82c75bB4a116F316Def)),
        _ => Err(WitnessError::UnsupportedChainError(chain.to_string())),
    }
}

pub(super) fn witness_hub(chain: Chain) -> Result<H160, WitnessError> {
    match chain {
        Chain::Mainnet => Ok(h160!(0xD25c2c5802198CB8541987b73A8db4c9BCaE5cC7)),
        Chain::Holesky => Ok(h160!(0xa987EC494b13b21A8a124F8Ac03c9F530648C87D)),
        _ => Err(WitnessError::UnsupportedChainError(chain.to_string())),
    }
}

pub(super) fn avs_directory(chain: Chain) -> Result<H160, WitnessError> {
    match chain {
        Chain::Mainnet => Ok(h160!(0x135dda560e946695d6f155dacafc6f1f25c1f5af)),
        Chain::Holesky => Ok(h160!(0x055733000064333CaDDbC92763c58BF0192fFeBf)),
        _ => Err(WitnessError::UnsupportedChainError(chain.to_string())),
    }
}
