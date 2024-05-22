use ethers_core::types::Address;
use once_cell::sync::Lazy;
use std::error::Error;

use super::{EigenStrategy, Strategy, StrategyError};

pub static MAINNET_LST_STRATEGIES: Lazy<Vec<Strategy>> = Lazy::new(|| {
    vec![
        Strategy::new("Cbeth", "0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc".parse().unwrap()),
        Strategy::new("Steth", "0x93c4b944D05dfe6df7645A86cd2206016c51564D".parse().unwrap()),
        Strategy::new("Reth", "0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2".parse().unwrap()),
        Strategy::new("Sweth", "0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6".parse().unwrap()),
        Strategy::new("Lseth", "0xAe60d8180437b5C34bB956822ac2710972584473".parse().unwrap()),
        Strategy::new("Sfrxeth", "0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6".parse().unwrap()),
        Strategy::new("Wbeth", "0x7CA911E83dabf90C90dD3De5411a10F1A6112184".parse().unwrap()),
        Strategy::new("Ethx", "0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d".parse().unwrap()),
        Strategy::new("Oseth", "0x57ba429517c3473B6d34CA9aCd56c0e735b94c02".parse().unwrap()),
        Strategy::new("Meth", "0x298aFB19A105D59E74658C4C334Ff360BadE6dd2".parse().unwrap()),
        Strategy::new("Ankreth", "0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff".parse().unwrap()),
        Strategy::new("Oeth", "0xa4C637e0F704745D182e4D38cAb7E7485321d059".parse().unwrap()),
        Strategy::new("BeaconEth", "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0".parse().unwrap()),
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
            MainnetLstStrategies::Cbeth => "0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc".parse().unwrap(),
            MainnetLstStrategies::Steth => "0x93c4b944D05dfe6df7645A86cd2206016c51564D".parse().unwrap(),
            MainnetLstStrategies::Reth => "0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2".parse().unwrap(),
            MainnetLstStrategies::Sweth => "0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6".parse().unwrap(),
            MainnetLstStrategies::Lseth => "0xAe60d8180437b5C34bB956822ac2710972584473".parse().unwrap(),
            MainnetLstStrategies::Sfrxeth => "0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6".parse().unwrap(),
            MainnetLstStrategies::Wbeth => "0x7CA911E83dabf90C90dD3De5411a10F1A6112184".parse().unwrap(),
            MainnetLstStrategies::Ethx => "0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d".parse().unwrap(),
            MainnetLstStrategies::Oseth => "0x57ba429517c3473B6d34CA9aCd56c0e735b94c02".parse().unwrap(),
            MainnetLstStrategies::Meth => "0x298aFB19A105D59E74658C4C334Ff360BadE6dd2".parse().unwrap(),
            MainnetLstStrategies::Ankreth => "0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff".parse().unwrap(),
            MainnetLstStrategies::BeaconEth => "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0".parse().unwrap(),
            MainnetLstStrategies::Oeth => "0xa4C637e0F704745D182e4D38cAb7E7485321d059".parse().unwrap(),
        }
    }
}
