use ethers::types::{Address, H160};
use ivynet_macros::h160;
use once_cell::sync::Lazy;
use std::error::Error;

use super::{Strategy, StrategyError, StrategyList};

pub static HOLESKY_LST_STRATEGIES: Lazy<Vec<Strategy>> = Lazy::new(|| {
    vec![
        Strategy::new("Steth", h160!(0x7d704507b76571a51d9cae8addabbfd0ba0e63d3)),
        Strategy::new("Reth", h160!(0x3A8fBdf9e77DFc25d09741f51d3E181b25d0c4E0)),
        Strategy::new("Weth", h160!(0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9)),
        Strategy::new("Lseth", h160!(0x05037A81BD7B4C9E0F7B430f1F2A22c31a2FD943)),
        Strategy::new("Sfrxeth", h160!(0x9281ff96637710Cd9A5CAcce9c6FAD8C9F54631c)),
        Strategy::new("Ethx", h160!(0x31B6F59e1627cEfC9fA174aD03859fC337666af7)),
        Strategy::new("Oseth", h160!(0x46281E3B7fDcACdBa44CADf069a94a588Fd4C6Ef)),
        Strategy::new("Cbeth", h160!(0x70EB4D3c164a6B4A5f908D4FBb5a9cAfFb66bAB6)),
        Strategy::new("Meth", h160!(0xaccc5A86732BE85b5012e8614AF237801636F8e5)),
        Strategy::new("Ankreth", h160!(0x7673a47463F80c6a3553Db9E54c8cDcd5313d0ac)),
        Strategy::new("BeaconEth", h160!(0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0)),
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

impl StrategyList for HoleskyLstStrategies {
    // TODO: OPTIMIZATION: This should reference a constant, not parse on the fly as parsing is
    // expensive.
    fn address(&self) -> Address {
        match self {
            HoleskyLstStrategies::Steth => h160!(0x7d704507b76571a51d9cae8addabbfd0ba0e63d3),
            HoleskyLstStrategies::Reth => h160!(0x3A8fBdf9e77DFc25d09741f51d3E181b25d0c4E0),
            HoleskyLstStrategies::Weth => h160!(0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9),
            HoleskyLstStrategies::Lseth => h160!(0x05037A81BD7B4C9E0F7B430f1F2A22c31a2FD943),
            HoleskyLstStrategies::Sfrxeth => h160!(0x9281ff96637710Cd9A5CAcce9c6FAD8C9F54631c),
            HoleskyLstStrategies::Ethx => h160!(0x31B6F59e1627cEfC9fA174aD03859fC337666af7),
            HoleskyLstStrategies::Oseth => h160!(0x46281E3B7fDcACdBa44CADf069a94a588Fd4C6Ef),
            HoleskyLstStrategies::Cbeth => h160!(0x70EB4D3c164a6B4A5f908D4FBb5a9cAfFb66bAB6),
            HoleskyLstStrategies::Meth => h160!(0xaccc5A86732BE85b5012e8614AF237801636F8e5),
            HoleskyLstStrategies::Ankreth => h160!(0x7673a47463F80c6a3553Db9E54c8cDcd5313d0ac),
            HoleskyLstStrategies::BeaconEth => h160!(0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0),
        }
    }
}
