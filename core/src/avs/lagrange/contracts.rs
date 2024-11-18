use ethers::{
    contract::abigen,
    types::{Address, Chain, H160},
};
use ivynet_macros::h160;

pub type ZKMRStakeRegistry = ZKMRStakeRegistryAbi<IvyProvider>;

abigen!(
    ZKMRStakeRegistryAbi,
    "abi/lagrange/ZKMRStakeRegistry.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

pub(super) fn zkmr_stake_registry(chain: Chain) -> Address {
    match chain {
        Chain::Mainnet => h160!(0x8dcdCc50Cc00Fe898b037bF61cCf3bf9ba46f15C),
        Chain::Holesky => h160!(0xf724cDC7C40fd6B59590C624E8F0E5E3843b4BE4),
        _ => todo!("Unimplemented"),
    }
}

#[allow(clippy::all)]
pub(super) fn registry_coordinator(chain: Chain) -> Address {
    match chain {
        // TODO: TEMP WHILE WE REWORK THIS STRUCT
        _ => h160!(0x00000000000000000000000000000000DeaDBeef),
    }
}
