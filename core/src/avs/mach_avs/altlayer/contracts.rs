use crate::{
    avs::contracts::{RegistryCoordinatorAbi, StakeRegistryAbi},
    rpc_management::IvyProvider,
};
use ethers::types::{Address, Chain, H160};
use ivynet_macros::h160;

pub type AltlayerStakeRegistry = StakeRegistryAbi<IvyProvider>;
pub type AltlayerRegistryCoordinator = RegistryCoordinatorAbi<IvyProvider>;

/// AltLayer stake registry contracts: https://github.com/alt-research/mach-avs
pub(super) fn stake_registry(chain: Chain) -> Address {
    match chain {
        Chain::Mainnet => h160!(0x49296A7D4a76888370CB377CD909Cc73a2f71289),
        Chain::Holesky => h160!(0x0b3eE1aDc2944DCcBb817f7d77915C7d38F7B858),
        _ => todo!("Unimplemented"),
    }
}

/// AltLayer registry coordinator contracts: https://github.com/alt-research/mach-avs
pub(super) fn registry_coordinator(chain: Chain) -> Address {
    match chain {
        Chain::Mainnet => h160!(0x561be1AB42170a19f31645F774e6e3862B2139AA),
        Chain::Holesky => h160!(0x1eA7D160d325B289bF981e0D7aB6Bf3261a0FFf2),
        _ => todo!("Unimplemented"),
    }
}
