use ethers::types::{Address, H160};
use ivynet_macros::h160;
use once_cell::sync::Lazy;
use std::error::Error;

use super::{EigenStrategy, Strategy, StrategyError};

pub static MAINNET_LST_STRATEGIES: Lazy<Vec<Strategy>> = Lazy::new(|| {
    vec![
        Strategy::new("Cbeth", h160!(0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc)),
        Strategy::new("Steth", h160!(0x93c4b944D05dfe6df7645A86cd2206016c51564D)),
        Strategy::new("Reth", h160!(0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2)),
        Strategy::new("Sweth", h160!(0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6)),
        Strategy::new("Lseth", h160!(0xAe60d8180437b5C34bB956822ac2710972584473)),
        Strategy::new("Sfrxeth", h160!(0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6)),
        Strategy::new("Wbeth", h160!(0x7CA911E83dabf90C90dD3De5411a10F1A6112184)),
        Strategy::new("Ethx", h160!(0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d)),
        Strategy::new("Oseth", h160!(0x57ba429517c3473B6d34CA9aCd56c0e735b94c02)),
        Strategy::new("Meth", h160!(0x298aFB19A105D59E74658C4C334Ff360BadE6dd2)),
        Strategy::new("Ankreth", h160!(0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff)),
        Strategy::new("Oeth", h160!(0xa4C637e0F704745D182e4D38cAb7E7485321d059)),
        Strategy::new("BeaconEth", h160!(0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0)),
    ]
});

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub enum MainnetLstStrategies {
    Cbeth,
    Steth,
    Reth,
    Sweth,
    Lseth,
    Sfrxeth,
    Wbeth,
    Ethx,
    Oseth,
    Meth,
    Ankreth,
    Oeth,
    BeaconEth,
}

impl TryFrom<&str> for MainnetLstStrategies {
    type Error = Box<dyn Error>;
    fn try_from(hex: &str) -> Result<Self, Self::Error> {
        let res = match hex {
            "0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc" => MainnetLstStrategies::Cbeth,
            "0x93c4b944D05dfe6df7645A86cd2206016c51564D" => MainnetLstStrategies::Steth,
            "0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2" => MainnetLstStrategies::Reth,
            "0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6" => MainnetLstStrategies::Sweth,
            "0xAe60d8180437b5C34bB956822ac2710972584473" => MainnetLstStrategies::Lseth,
            "0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6" => MainnetLstStrategies::Sfrxeth,
            "0x7CA911E83dabf90C90dD3De5411a10F1A6112184" => MainnetLstStrategies::Wbeth,
            "0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d" => MainnetLstStrategies::Ethx,
            "0x57ba429517c3473B6d34CA9aCd56c0e735b94c02" => MainnetLstStrategies::Oseth,
            "0x298aFB19A105D59E74658C4C334Ff360BadE6dd2" => MainnetLstStrategies::Meth,
            "0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff" => MainnetLstStrategies::Ankreth,
            "0xa4C637e0F704745D182e4D38cAb7E7485321d059" => MainnetLstStrategies::Oeth,
            "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0" => MainnetLstStrategies::BeaconEth,

            _ => return Err(StrategyError::UnknownStrategy.into()),
        };
        Ok(res)
    }
}

impl EigenStrategy for MainnetLstStrategies {
    // TODO: OPTIMIZATION: This should reference a constant, not parse on the fly as parsing is
    // expensive.
    fn address(&self) -> Address {
        match self {
            MainnetLstStrategies::Cbeth => h160!(0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc),
            MainnetLstStrategies::Steth => h160!(0x93c4b944D05dfe6df7645A86cd2206016c51564D),
            MainnetLstStrategies::Reth => h160!(0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2),
            MainnetLstStrategies::Sweth => h160!(0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6),
            MainnetLstStrategies::Lseth => h160!(0xAe60d8180437b5C34bB956822ac2710972584473),
            MainnetLstStrategies::Sfrxeth => h160!(0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6),
            MainnetLstStrategies::Wbeth => h160!(0x7CA911E83dabf90C90dD3De5411a10F1A6112184),
            MainnetLstStrategies::Ethx => h160!(0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d),
            MainnetLstStrategies::Oseth => h160!(0x57ba429517c3473B6d34CA9aCd56c0e735b94c02),
            MainnetLstStrategies::Meth => h160!(0x298aFB19A105D59E74658C4C334Ff360BadE6dd2),
            MainnetLstStrategies::Ankreth => h160!(0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff),
            MainnetLstStrategies::BeaconEth => h160!(0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0),
            MainnetLstStrategies::Oeth => h160!(0xa4C637e0F704745D182e4D38cAb7E7485321d059),
        }
    }
}
