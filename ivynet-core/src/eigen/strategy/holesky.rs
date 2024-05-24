use ethers::types::Address;
use once_cell::sync::Lazy;
use std::error::Error;

use super::{EigenStrategy, Strategy, StrategyError};

pub static HOLESKY_LST_STRATEGIES: Lazy<Vec<Strategy>> = Lazy::new(|| {
    vec![
        Strategy::new("Steth", "0x7d704507b76571a51d9cae8addabbfd0ba0e63d3".parse().unwrap()),
        Strategy::new("Reth", "0x3A8fBdf9e77DFc25d09741f51d3E181b25d0c4E0".parse().unwrap()),
        Strategy::new("Weth", "0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9".parse().unwrap()),
        Strategy::new("Lseth", "0x05037A81BD7B4C9E0F7B430f1F2A22c31a2FD943".parse().unwrap()),
        Strategy::new("Sfrxeth", "0x9281ff96637710Cd9A5CAcce9c6FAD8C9F54631c".parse().unwrap()),
        Strategy::new("Ethx", "0x31B6F59e1627cEfC9fA174aD03859fC337666af7".parse().unwrap()),
        Strategy::new("Oseth", "0x46281E3B7fDcACdBa44CADf069a94a588Fd4C6Ef".parse().unwrap()),
        Strategy::new("Cbeth", "0x70EB4D3c164a6B4A5f908D4FBb5a9cAfFb66bAB6".parse().unwrap()),
        Strategy::new("Meth", "0xaccc5A86732BE85b5012e8614AF237801636F8e5".parse().unwrap()),
        Strategy::new("Ankreth", "0x7673a47463F80c6a3553Db9E54c8cDcd5313d0ac".parse().unwrap()),
        Strategy::new("BeaconEth", "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0".parse().unwrap()),
    ]
});

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub enum HoleskyLstStrategies {
    Steth,
    Reth,
    Weth,
    Lseth,
    Sfrxeth,
    Ethx,
    Oseth,
    Cbeth,
    Meth,
    Ankreth,
    BeaconEth,
}

impl TryFrom<&str> for HoleskyLstStrategies {
    type Error = Box<dyn Error>;
    fn try_from(hex: &str) -> Result<Self, Self::Error> {
        let res = match hex {
            "0x7d704507b76571a51d9cae8addabbfd0ba0e63d3" => HoleskyLstStrategies::Steth,
            "0x3A8fBdf9e77DFc25d09741f51d3E181b25d0c4E0" => HoleskyLstStrategies::Reth,
            "0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9" => HoleskyLstStrategies::Weth,
            "0x05037A81BD7B4C9E0F7B430f1F2A22c31a2FD943" => HoleskyLstStrategies::Lseth,
            "0x9281ff96637710Cd9A5CAcce9c6FAD8C9F54631c" => HoleskyLstStrategies::Sfrxeth,
            "0x31B6F59e1627cEfC9fA174aD03859fC337666af7" => HoleskyLstStrategies::Ethx,
            "0x46281E3B7fDcACdBa44CADf069a94a588Fd4C6Ef" => HoleskyLstStrategies::Oseth,
            "0x70EB4D3c164a6B4A5f908D4FBb5a9cAfFb66bAB6" => HoleskyLstStrategies::Cbeth,
            "0xaccc5A86732BE85b5012e8614AF237801636F8e5" => HoleskyLstStrategies::Meth,
            "0x7673a47463F80c6a3553Db9E54c8cDcd5313d0ac" => HoleskyLstStrategies::Ankreth,
            "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0" => HoleskyLstStrategies::BeaconEth,
            _ => return Err(StrategyError::UnknownStrategy.into()),
        };
        Ok(res)
    }
}

impl EigenStrategy for HoleskyLstStrategies {
    // TODO: OPTIMIZATION: This should reference a constant, not parse on the fly as parsing is
    // expensive.
    fn address(&self) -> Address {
        match self {
            HoleskyLstStrategies::Steth => "0x7d704507b76571a51d9cae8addabbfd0ba0e63d3".parse().unwrap(),
            HoleskyLstStrategies::Reth => "0x3A8fBdf9e77DFc25d09741f51d3E181b25d0c4E0".parse().unwrap(),
            HoleskyLstStrategies::Weth => "0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9".parse().unwrap(),
            HoleskyLstStrategies::Lseth => "0x05037A81BD7B4C9E0F7B430f1F2A22c31a2FD943".parse().unwrap(),
            HoleskyLstStrategies::Sfrxeth => "0x9281ff96637710Cd9A5CAcce9c6FAD8C9F54631c".parse().unwrap(),
            HoleskyLstStrategies::Ethx => "0x31B6F59e1627cEfC9fA174aD03859fC337666af7".parse().unwrap(),
            HoleskyLstStrategies::Oseth => "0x46281E3B7fDcACdBa44CADf069a94a588Fd4C6Ef".parse().unwrap(),
            HoleskyLstStrategies::Cbeth => "0x70EB4D3c164a6B4A5f908D4FBb5a9cAfFb66bAB6".parse().unwrap(),
            HoleskyLstStrategies::Meth => "0xaccc5A86732BE85b5012e8614AF237801636F8e5".parse().unwrap(),
            HoleskyLstStrategies::Ankreth => "0x7673a47463F80c6a3553Db9E54c8cDcd5313d0ac".parse().unwrap(),
            HoleskyLstStrategies::BeaconEth => "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0".parse().unwrap(),
        }
    }
}
