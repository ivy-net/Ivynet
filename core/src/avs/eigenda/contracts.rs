use crate::IvyProvider;
use ethers::{
    contract::abigen,
    types::{Address, Chain, H160},
};
use ivynet_macros::h160;

#[allow(dead_code)]
pub type StakeRegistry = StakeRegistryAbi<IvyProvider>;
#[allow(dead_code)]
pub type RegistryCoordinator = RegistryCoordinatorAbi<IvyProvider>;

abigen!(
    StakeRegistryAbi,
    "abi/eigenda/StakeRegistry.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    RegistryCoordinatorAbi,
    "abi/eigenda/RegistryCoordinator.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

#[allow(dead_code)]
pub(super) fn stake_registry(chain: Chain) -> Address {
    match chain {
        Chain::Mainnet => h160!(0x006124ae7976137266feebfb3f4d2be4c073139d),
        Chain::Holesky => h160!(0xBDACD5998989Eec814ac7A0f0f6596088AA2a270),
        _ => todo!("Unimplemented"),
    }
}

#[allow(dead_code)]
pub(super) fn registry_coordinator(chain: Chain) -> Address {
    match chain {
        Chain::Mainnet => h160!(0x0baac79acd45a023e19345c352d8a7a83c4e5656),
        Chain::Holesky => h160!(0x53012C69A189cfA2D9d29eb6F19B32e0A2EA3490),
        _ => todo!("Unimplemented"),
    }
}

#[allow(dead_code)]
pub(super) fn delegation_manager(chain: Chain) -> Address {
    match chain {
        Chain::Mainnet => h160!(0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A),
        Chain::Holesky => h160!(0xA44151489861Fe9e3055d95adC98FbD462B948e7),
        _ => todo!("Unimplemented"),
    }
}
