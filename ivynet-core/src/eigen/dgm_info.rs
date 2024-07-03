use ethers::{
    contract::abigen,
    types::{Chain, H160},
};
use ivynet_macros::h160;

// EigenLayer shares types in order of their appearance on EL website
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum EigenStrategy {
    Weth,
    BeaconEth,
    Reth,
    Oseth,
    Steth,
    Ankreth,
    Meth,
    Ethx,
    Lseth,
    Cbeth,
    Sfrxeth,
    Sweth,
    Oeth,
    Wbeth,
    Unknown,
}

pub fn get_delegation_manager_address(chain: Chain) -> H160 {
    match chain {
        Chain::Holesky => h160!(0xA44151489861Fe9e3055d95adC98FbD462B948e7),
        Chain::Mainnet => h160!(0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A),
        //Fork testnet
        Chain::AnvilHardhat => h160!(0xA44151489861Fe9e3055d95adC98FbD462B948e7),
        _ => panic!("Chain not supported"),
    }
}

pub fn get_strategy_list(chain: Chain) -> Vec<EigenStrategy> {
    match chain {
        Chain::Holesky => vec![
            EigenStrategy::Steth,
            EigenStrategy::Reth,
            EigenStrategy::Weth,
            EigenStrategy::Lseth,
            EigenStrategy::Sfrxeth,
            EigenStrategy::Ethx,
            EigenStrategy::Oseth,
            EigenStrategy::Cbeth,
            EigenStrategy::Meth,
            EigenStrategy::Ankreth,
            EigenStrategy::BeaconEth,
        ],
        Chain::Mainnet => vec![
            EigenStrategy::Cbeth,
            EigenStrategy::Steth,
            EigenStrategy::Reth,
            EigenStrategy::Sweth,
            EigenStrategy::Lseth,
            EigenStrategy::Sfrxeth,
            EigenStrategy::Wbeth,
            EigenStrategy::Ethx,
            EigenStrategy::Oseth,
            EigenStrategy::Meth,
            EigenStrategy::Ankreth,
            EigenStrategy::BeaconEth,
            EigenStrategy::Oeth,
        ],
        _ => todo!(),
    }
}

// https://github.com/Layr-Labs/eigenlayer-contracts/blob/dev/src/contracts/interfaces/IDelegationManager.sol
abigen!(DelegationManagerAbi, "abi/DelegationManager.json", event_derives(serde::Deserialize, serde::Serialize));
