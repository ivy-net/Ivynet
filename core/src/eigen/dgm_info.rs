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

// impl From<&str> for EigenStrategy {
//     fn from(hex: &str) -> Self {
//         match rpc_management::get_network() {
//             Network::Holesky => match hex {
//                 "0x7d704507b76571a51d9cae8addabbfd0ba0e63d3" => EigenStrategy::Steth,
//                 "0x3A8fBdf9e77DFc25d09741f51d3E181b25d0c4E0" => EigenStrategy::Reth,
//                 "0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9" => EigenStrategy::Weth,
//                 "0x05037A81BD7B4C9E0F7B430f1F2A22c31a2FD943" => EigenStrategy::Lseth,
//                 "0x9281ff96637710Cd9A5CAcce9c6FAD8C9F54631c" => EigenStrategy::Sfrxeth,
//                 "0x31B6F59e1627cEfC9fA174aD03859fC337666af7" => EigenStrategy::Ethx,
//                 "0x46281E3B7fDcACdBa44CADf069a94a588Fd4C6Ef" => EigenStrategy::Oseth,
//                 "0x70EB4D3c164a6B4A5f908D4FBb5a9cAfFb66bAB6" => EigenStrategy::Cbeth,
//                 "0xaccc5A86732BE85b5012e8614AF237801636F8e5" => EigenStrategy::Meth,
//                 "0x7673a47463F80c6a3553Db9E54c8cDcd5313d0ac" => EigenStrategy::Ankreth,
//                 "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0" => EigenStrategy::BeaconEth,
//                 _ => EigenStrategy::Unknown,
//             },
//             Network::Mainnet => match hex {
//                 "0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc" => EigenStrategy::Cbeth,
//                 "0x93c4b944D05dfe6df7645A86cd2206016c51564D" => EigenStrategy::Steth,
//                 "0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2" => EigenStrategy::Reth,
//                 "0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6" => EigenStrategy::Sweth,
//                 "0xAe60d8180437b5C34bB956822ac2710972584473" => EigenStrategy::Lseth,
//                 "0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6" => EigenStrategy::Sfrxeth,
//                 "0x7CA911E83dabf90C90dD3De5411a10F1A6112184" => EigenStrategy::Wbeth,
//                 "0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d" => EigenStrategy::Ethx,
//                 "0x57ba429517c3473B6d34CA9aCd56c0e735b94c02" => EigenStrategy::Oseth,
//                 "0x298aFB19A105D59E74658C4C334Ff360BadE6dd2" => EigenStrategy::Meth,
//                 "0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff" => EigenStrategy::Ankreth,
//                 "0xa4C637e0F704745D182e4D38cAb7E7485321d059" => EigenStrategy::Oeth,
//                 "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0" => EigenStrategy::BeaconEth,
//                 _ => EigenStrategy::Unknown,
//             },
//             Network::Local => todo!(),
//         }
//     }
// }

// impl From<EigenStrategy> for Address {
//     fn from(strategy: EigenStrategy) -> Self {
//         match rpc_management::get_network() {
//             Network::Holesky => match strategy {
//                 EigenStrategy::Steth =>
// "0x7d704507b76571a51d9cae8addabbfd0ba0e63d3".parse().unwrap(),
// EigenStrategy::Reth => "0x3A8fBdf9e77DFc25d09741f51d3E181b25d0c4E0".parse().unwrap(),
// EigenStrategy::Weth => "0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9".parse().unwrap(),
// EigenStrategy::Lseth => "0x05037A81BD7B4C9E0F7B430f1F2A22c31a2FD943".parse().unwrap(),
//                 EigenStrategy::Sfrxeth =>
// "0x9281ff96637710Cd9A5CAcce9c6FAD8C9F54631c".parse().unwrap(),
// EigenStrategy::Ethx => "0x31B6F59e1627cEfC9fA174aD03859fC337666af7".parse().unwrap(),
// EigenStrategy::Oseth => "0x46281E3B7fDcACdBa44CADf069a94a588Fd4C6Ef".parse().unwrap(),
//                 EigenStrategy::Cbeth =>
// "0x70EB4D3c164a6B4A5f908D4FBb5a9cAfFb66bAB6".parse().unwrap(),
// EigenStrategy::Meth => "0xaccc5A86732BE85b5012e8614AF237801636F8e5".parse().unwrap(),
// EigenStrategy::Ankreth => "0x7673a47463F80c6a3553Db9E54c8cDcd5313d0ac".parse().unwrap(),
//                 EigenStrategy::BeaconEth =>
// "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0".parse().unwrap(),                 _ =>
// "".parse().unwrap(), // Panics             },
//             Network::Mainnet => match strategy {
//                 EigenStrategy::Cbeth =>
// "0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc".parse().unwrap(),
// EigenStrategy::Steth => "0x93c4b944D05dfe6df7645A86cd2206016c51564D".parse().unwrap(),
//                 EigenStrategy::Reth =>
// "0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2".parse().unwrap(),
// EigenStrategy::Sweth => "0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6".parse().unwrap(),
//                 EigenStrategy::Lseth =>
// "0xAe60d8180437b5C34bB956822ac2710972584473".parse().unwrap(),
// EigenStrategy::Sfrxeth => "0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6".parse().unwrap(),
//                 EigenStrategy::Wbeth =>
// "0x7CA911E83dabf90C90dD3De5411a10F1A6112184".parse().unwrap(),
// EigenStrategy::Ethx => "0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d".parse().unwrap(),
// EigenStrategy::Oseth => "0x57ba429517c3473B6d34CA9aCd56c0e735b94c02".parse().unwrap(),
//                 EigenStrategy::Meth =>
// "0x298aFB19A105D59E74658C4C334Ff360BadE6dd2".parse().unwrap(),
// EigenStrategy::Ankreth => "0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff".parse().unwrap(),
//                 EigenStrategy::BeaconEth =>
// "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0".parse().unwrap(),
// EigenStrategy::Oeth => "0xa4C637e0F704745D182e4D38cAb7E7485321d059".parse().unwrap(),
// _ => "".parse().unwrap(), // Panics             },
//             Network::Local => todo!(),
//         }
//     }
// }

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
