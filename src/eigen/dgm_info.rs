use ethers_contract::abigen;

use crate::rpc_management::{self, Network};

lazy_static::lazy_static! {
    pub static ref NETWORK: Network = rpc_management::NETWORK.lock().unwrap().clone();
    pub static ref DELEGATION_MANAGER_ADDRESS: String = get_delegation_manager_address();
    pub static ref STRATEGY_LIST: Vec<EigenStrategy> = get_strategy_list();
}

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

impl From<&str> for EigenStrategy {
    fn from(hex: &str) -> Self {
        match NETWORK.clone() {
            Network::Holesky => match hex {
                "0x7d704507b76571a51d9cae8addabbfd0ba0e63d3" => EigenStrategy::Steth,
                "0x3A8fBdf9e77DFc25d09741f51d3E181b25d0c4E0" => EigenStrategy::Reth,
                "0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9" => EigenStrategy::Weth,
                "0x05037A81BD7B4C9E0F7B430f1F2A22c31a2FD943" => EigenStrategy::Lseth,
                "0x9281ff96637710Cd9A5CAcce9c6FAD8C9F54631c" => EigenStrategy::Sfrxeth,
                "0x31B6F59e1627cEfC9fA174aD03859fC337666af7" => EigenStrategy::Ethx,
                "0x46281E3B7fDcACdBa44CADf069a94a588Fd4C6Ef" => EigenStrategy::Oseth,
                "0x70EB4D3c164a6B4A5f908D4FBb5a9cAfFb66bAB6" => EigenStrategy::Cbeth,
                "0xaccc5A86732BE85b5012e8614AF237801636F8e5" => EigenStrategy::Meth,
                "0x7673a47463F80c6a3553Db9E54c8cDcd5313d0ac" => EigenStrategy::Ankreth,
                "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0" => EigenStrategy::BeaconEth,
                _ => EigenStrategy::Unknown,
            },
            Network::Mainnet => match hex {
                "0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc" => EigenStrategy::Cbeth,
                "0x93c4b944D05dfe6df7645A86cd2206016c51564D" => EigenStrategy::Steth,
                "0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2" => EigenStrategy::Reth,
                "0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6" => EigenStrategy::Sweth,
                "0xAe60d8180437b5C34bB956822ac2710972584473" => EigenStrategy::Lseth,
                "0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6" => EigenStrategy::Sfrxeth,
                "0x7CA911E83dabf90C90dD3De5411a10F1A6112184" => EigenStrategy::Wbeth,
                "0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d" => EigenStrategy::Ethx,
                "0x57ba429517c3473B6d34CA9aCd56c0e735b94c02" => EigenStrategy::Oseth,
                "0x298aFB19A105D59E74658C4C334Ff360BadE6dd2" => EigenStrategy::Meth,
                "0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff" => EigenStrategy::Ankreth,
                "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0" => EigenStrategy::BeaconEth,
                "0xa4C637e0F704745D182e4D38cAb7E7485321d059" => EigenStrategy::Oeth,
                _ => EigenStrategy::Unknown,
            },
            Network::Local => todo!(),
        }
    }
}

impl From<EigenStrategy> for &str {
    fn from(strategy: EigenStrategy) -> Self {
        match NETWORK.clone() {
            Network::Holesky => match strategy {
                EigenStrategy::Steth => "0x7d704507b76571a51d9cae8addabbfd0ba0e63d3",
                EigenStrategy::Reth => "0x3A8fBdf9e77DFc25d09741f51d3E181b25d0c4E0",
                EigenStrategy::Weth => "0x80528D6e9A2BAbFc766965E0E26d5aB08D9CFaF9",
                EigenStrategy::Lseth => "0x05037A81BD7B4C9E0F7B430f1F2A22c31a2FD943",
                EigenStrategy::Sfrxeth => "0x9281ff96637710Cd9A5CAcce9c6FAD8C9F54631c",
                EigenStrategy::Ethx => "0x31B6F59e1627cEfC9fA174aD03859fC337666af7",
                EigenStrategy::Oseth => "0x46281E3B7fDcACdBa44CADf069a94a588Fd4C6Ef",
                EigenStrategy::Cbeth => "0x70EB4D3c164a6B4A5f908D4FBb5a9cAfFb66bAB6",
                EigenStrategy::Meth => "0xaccc5A86732BE85b5012e8614AF237801636F8e5",
                EigenStrategy::Ankreth => "0x7673a47463F80c6a3553Db9E54c8cDcd5313d0ac",
                EigenStrategy::BeaconEth => "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0",
                _ => "",
            },
            Network::Mainnet => match strategy {
                EigenStrategy::Cbeth => "0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc",
                EigenStrategy::Steth => "0x93c4b944D05dfe6df7645A86cd2206016c51564D",
                EigenStrategy::Reth => "0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2",
                EigenStrategy::Sweth => "0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6",
                EigenStrategy::Lseth => "0xAe60d8180437b5C34bB956822ac2710972584473",
                EigenStrategy::Sfrxeth => "0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6",
                EigenStrategy::Wbeth => "0x7CA911E83dabf90C90dD3De5411a10F1A6112184",
                EigenStrategy::Ethx => "0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d",
                EigenStrategy::Oseth => "0x57ba429517c3473B6d34CA9aCd56c0e735b94c02",
                EigenStrategy::Meth => "0x298aFB19A105D59E74658C4C334Ff360BadE6dd2",
                EigenStrategy::Ankreth => "0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff",
                EigenStrategy::BeaconEth => "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0",
                EigenStrategy::Oeth => "0xa4C637e0F704745D182e4D38cAb7E7485321d059",
                _ => "",
            },
            Network::Local => todo!(),
        }
    }
}

fn get_delegation_manager_address() -> String {
    match NETWORK.clone() {
        Network::Holesky => "0xA44151489861Fe9e3055d95adC98FbD462B948e7".to_string(),
        Network::Mainnet => "0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A".to_string(),
        Network::Local => todo!(),
    }
}

fn get_strategy_list() -> Vec<EigenStrategy> {
    match NETWORK.clone() {
        Network::Holesky => vec![
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
        Network::Mainnet => vec![
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
        Network::Local => todo!(),
    }
}

abigen!(
    DelegationManagerAbi,
    r#"[{"type":"constructor","inputs":[{"name":"_strategyManager","type":"address","internalType":"contract IStrategyManager"},{"name":"_slasher","type":"address","internalType":"contract ISlasher"},{"name":"_eigenPodManager","type":"address","internalType":"contract IEigenPodManager"}],"stateMutability":"nonpayable"},{"type":"function","name":"DELEGATION_APPROVAL_TYPEHASH","inputs":[],"outputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"stateMutability":"view"},{"type":"function","name":"DOMAIN_TYPEHASH","inputs":[],"outputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"stateMutability":"view"},{"type":"function","name":"MAX_STAKER_OPT_OUT_WINDOW_BLOCKS","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"MAX_WITHDRAWAL_DELAY_BLOCKS","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"STAKER_DELEGATION_TYPEHASH","inputs":[],"outputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"stateMutability":"view"},{"type":"function","name":"beaconChainETHStrategy","inputs":[],"outputs":[{"name":"","type":"address","internalType":"contract IStrategy"}],"stateMutability":"view"},{"type":"function","name":"calculateCurrentStakerDelegationDigestHash","inputs":[{"name":"staker","type":"address","internalType":"address"},{"name":"operator","type":"address","internalType":"address"},{"name":"expiry","type":"uint256","internalType":"uint256"}],"outputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"stateMutability":"view"},{"type":"function","name":"calculateDelegationApprovalDigestHash","inputs":[{"name":"staker","type":"address","internalType":"address"},{"name":"operator","type":"address","internalType":"address"},{"name":"_delegationApprover","type":"address","internalType":"address"},{"name":"approverSalt","type":"bytes32","internalType":"bytes32"},{"name":"expiry","type":"uint256","internalType":"uint256"}],"outputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"stateMutability":"view"},{"type":"function","name":"calculateStakerDelegationDigestHash","inputs":[{"name":"staker","type":"address","internalType":"address"},{"name":"_stakerNonce","type":"uint256","internalType":"uint256"},{"name":"operator","type":"address","internalType":"address"},{"name":"expiry","type":"uint256","internalType":"uint256"}],"outputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"stateMutability":"view"},{"type":"function","name":"calculateWithdrawalRoot","inputs":[{"name":"withdrawal","type":"tuple","internalType":"struct IDelegationManager.Withdrawal","components":[{"name":"staker","type":"address","internalType":"address"},{"name":"delegatedTo","type":"address","internalType":"address"},{"name":"withdrawer","type":"address","internalType":"address"},{"name":"nonce","type":"uint256","internalType":"uint256"},{"name":"startBlock","type":"uint32","internalType":"uint32"},{"name":"strategies","type":"address[]","internalType":"contract IStrategy[]"},{"name":"shares","type":"uint256[]","internalType":"uint256[]"}]}],"outputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"stateMutability":"pure"},{"type":"function","name":"completeQueuedWithdrawal","inputs":[{"name":"withdrawal","type":"tuple","internalType":"struct IDelegationManager.Withdrawal","components":[{"name":"staker","type":"address","internalType":"address"},{"name":"delegatedTo","type":"address","internalType":"address"},{"name":"withdrawer","type":"address","internalType":"address"},{"name":"nonce","type":"uint256","internalType":"uint256"},{"name":"startBlock","type":"uint32","internalType":"uint32"},{"name":"strategies","type":"address[]","internalType":"contract IStrategy[]"},{"name":"shares","type":"uint256[]","internalType":"uint256[]"}]},{"name":"tokens","type":"address[]","internalType":"contract IERC20[]"},{"name":"middlewareTimesIndex","type":"uint256","internalType":"uint256"},{"name":"receiveAsTokens","type":"bool","internalType":"bool"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"completeQueuedWithdrawals","inputs":[{"name":"withdrawals","type":"tuple[]","internalType":"struct IDelegationManager.Withdrawal[]","components":[{"name":"staker","type":"address","internalType":"address"},{"name":"delegatedTo","type":"address","internalType":"address"},{"name":"withdrawer","type":"address","internalType":"address"},{"name":"nonce","type":"uint256","internalType":"uint256"},{"name":"startBlock","type":"uint32","internalType":"uint32"},{"name":"strategies","type":"address[]","internalType":"contract IStrategy[]"},{"name":"shares","type":"uint256[]","internalType":"uint256[]"}]},{"name":"tokens","type":"address[][]","internalType":"contract IERC20[][]"},{"name":"middlewareTimesIndexes","type":"uint256[]","internalType":"uint256[]"},{"name":"receiveAsTokens","type":"bool[]","internalType":"bool[]"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"cumulativeWithdrawalsQueued","inputs":[{"name":"","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"decreaseDelegatedShares","inputs":[{"name":"staker","type":"address","internalType":"address"},{"name":"strategy","type":"address","internalType":"contract IStrategy"},{"name":"shares","type":"uint256","internalType":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"delegateTo","inputs":[{"name":"operator","type":"address","internalType":"address"},{"name":"approverSignatureAndExpiry","type":"tuple","internalType":"struct ISignatureUtils.SignatureWithExpiry","components":[{"name":"signature","type":"bytes","internalType":"bytes"},{"name":"expiry","type":"uint256","internalType":"uint256"}]},{"name":"approverSalt","type":"bytes32","internalType":"bytes32"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"delegateToBySignature","inputs":[{"name":"staker","type":"address","internalType":"address"},{"name":"operator","type":"address","internalType":"address"},{"name":"stakerSignatureAndExpiry","type":"tuple","internalType":"struct ISignatureUtils.SignatureWithExpiry","components":[{"name":"signature","type":"bytes","internalType":"bytes"},{"name":"expiry","type":"uint256","internalType":"uint256"}]},{"name":"approverSignatureAndExpiry","type":"tuple","internalType":"struct ISignatureUtils.SignatureWithExpiry","components":[{"name":"signature","type":"bytes","internalType":"bytes"},{"name":"expiry","type":"uint256","internalType":"uint256"}]},{"name":"approverSalt","type":"bytes32","internalType":"bytes32"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"delegatedTo","inputs":[{"name":"","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"delegationApprover","inputs":[{"name":"operator","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"delegationApproverSaltIsSpent","inputs":[{"name":"","type":"address","internalType":"address"},{"name":"","type":"bytes32","internalType":"bytes32"}],"outputs":[{"name":"","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"domainSeparator","inputs":[],"outputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"stateMutability":"view"},{"type":"function","name":"earningsReceiver","inputs":[{"name":"operator","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"eigenPodManager","inputs":[],"outputs":[{"name":"","type":"address","internalType":"contract IEigenPodManager"}],"stateMutability":"view"},{"type":"function","name":"getDelegatableShares","inputs":[{"name":"staker","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"address[]","internalType":"contract IStrategy[]"},{"name":"","type":"uint256[]","internalType":"uint256[]"}],"stateMutability":"view"},{"type":"function","name":"getOperatorShares","inputs":[{"name":"operator","type":"address","internalType":"address"},{"name":"strategies","type":"address[]","internalType":"contract IStrategy[]"}],"outputs":[{"name":"","type":"uint256[]","internalType":"uint256[]"}],"stateMutability":"view"},{"type":"function","name":"getWithdrawalDelay","inputs":[{"name":"strategies","type":"address[]","internalType":"contract IStrategy[]"}],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"increaseDelegatedShares","inputs":[{"name":"staker","type":"address","internalType":"address"},{"name":"strategy","type":"address","internalType":"contract IStrategy"},{"name":"shares","type":"uint256","internalType":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"initialize","inputs":[{"name":"initialOwner","type":"address","internalType":"address"},{"name":"_pauserRegistry","type":"address","internalType":"contract IPauserRegistry"},{"name":"initialPausedStatus","type":"uint256","internalType":"uint256"},{"name":"_minWithdrawalDelayBlocks","type":"uint256","internalType":"uint256"},{"name":"_strategies","type":"address[]","internalType":"contract IStrategy[]"},{"name":"_withdrawalDelayBlocks","type":"uint256[]","internalType":"uint256[]"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"isDelegated","inputs":[{"name":"staker","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"isOperator","inputs":[{"name":"operator","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"migrateQueuedWithdrawals","inputs":[{"name":"withdrawalsToMigrate","type":"tuple[]","internalType":"struct IStrategyManager.DeprecatedStruct_QueuedWithdrawal[]","components":[{"name":"strategies","type":"address[]","internalType":"contract IStrategy[]"},{"name":"shares","type":"uint256[]","internalType":"uint256[]"},{"name":"staker","type":"address","internalType":"address"},{"name":"withdrawerAndNonce","type":"tuple","internalType":"struct IStrategyManager.DeprecatedStruct_WithdrawerAndNonce","components":[{"name":"withdrawer","type":"address","internalType":"address"},{"name":"nonce","type":"uint96","internalType":"uint96"}]},{"name":"withdrawalStartBlock","type":"uint32","internalType":"uint32"},{"name":"delegatedAddress","type":"address","internalType":"address"}]}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"minWithdrawalDelayBlocks","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"modifyOperatorDetails","inputs":[{"name":"newOperatorDetails","type":"tuple","internalType":"struct IDelegationManager.OperatorDetails","components":[{"name":"earningsReceiver","type":"address","internalType":"address"},{"name":"delegationApprover","type":"address","internalType":"address"},{"name":"stakerOptOutWindowBlocks","type":"uint32","internalType":"uint32"}]}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"operatorDetails","inputs":[{"name":"operator","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"tuple","internalType":"struct IDelegationManager.OperatorDetails","components":[{"name":"earningsReceiver","type":"address","internalType":"address"},{"name":"delegationApprover","type":"address","internalType":"address"},{"name":"stakerOptOutWindowBlocks","type":"uint32","internalType":"uint32"}]}],"stateMutability":"view"},{"type":"function","name":"operatorShares","inputs":[{"name":"","type":"address","internalType":"address"},{"name":"","type":"address","internalType":"contract IStrategy"}],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"owner","inputs":[],"outputs":[{"name":"","type":"address","internalType":"address"}],"stateMutability":"view"},{"type":"function","name":"pause","inputs":[{"name":"newPausedStatus","type":"uint256","internalType":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"pauseAll","inputs":[],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"paused","inputs":[{"name":"index","type":"uint8","internalType":"uint8"}],"outputs":[{"name":"","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"paused","inputs":[],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"pauserRegistry","inputs":[],"outputs":[{"name":"","type":"address","internalType":"contract IPauserRegistry"}],"stateMutability":"view"},{"type":"function","name":"pendingWithdrawals","inputs":[{"name":"","type":"bytes32","internalType":"bytes32"}],"outputs":[{"name":"","type":"bool","internalType":"bool"}],"stateMutability":"view"},{"type":"function","name":"queueWithdrawals","inputs":[{"name":"queuedWithdrawalParams","type":"tuple[]","internalType":"struct IDelegationManager.QueuedWithdrawalParams[]","components":[{"name":"strategies","type":"address[]","internalType":"contract IStrategy[]"},{"name":"shares","type":"uint256[]","internalType":"uint256[]"},{"name":"withdrawer","type":"address","internalType":"address"}]}],"outputs":[{"name":"","type":"bytes32[]","internalType":"bytes32[]"}],"stateMutability":"nonpayable"},{"type":"function","name":"registerAsOperator","inputs":[{"name":"registeringOperatorDetails","type":"tuple","internalType":"struct IDelegationManager.OperatorDetails","components":[{"name":"earningsReceiver","type":"address","internalType":"address"},{"name":"delegationApprover","type":"address","internalType":"address"},{"name":"stakerOptOutWindowBlocks","type":"uint32","internalType":"uint32"}]},{"name":"metadataURI","type":"string","internalType":"string"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"renounceOwnership","inputs":[],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"setMinWithdrawalDelayBlocks","inputs":[{"name":"newMinWithdrawalDelayBlocks","type":"uint256","internalType":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"setPauserRegistry","inputs":[{"name":"newPauserRegistry","type":"address","internalType":"contract IPauserRegistry"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"setStrategyWithdrawalDelayBlocks","inputs":[{"name":"strategies","type":"address[]","internalType":"contract IStrategy[]"},{"name":"withdrawalDelayBlocks","type":"uint256[]","internalType":"uint256[]"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"slasher","inputs":[],"outputs":[{"name":"","type":"address","internalType":"contract ISlasher"}],"stateMutability":"view"},{"type":"function","name":"stakerNonce","inputs":[{"name":"","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"stakerOptOutWindowBlocks","inputs":[{"name":"operator","type":"address","internalType":"address"}],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"strategyManager","inputs":[],"outputs":[{"name":"","type":"address","internalType":"contract IStrategyManager"}],"stateMutability":"view"},{"type":"function","name":"strategyWithdrawalDelayBlocks","inputs":[{"name":"","type":"address","internalType":"contract IStrategy"}],"outputs":[{"name":"","type":"uint256","internalType":"uint256"}],"stateMutability":"view"},{"type":"function","name":"transferOwnership","inputs":[{"name":"newOwner","type":"address","internalType":"address"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"undelegate","inputs":[{"name":"staker","type":"address","internalType":"address"}],"outputs":[{"name":"withdrawalRoots","type":"bytes32[]","internalType":"bytes32[]"}],"stateMutability":"nonpayable"},{"type":"function","name":"unpause","inputs":[{"name":"newPausedStatus","type":"uint256","internalType":"uint256"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"function","name":"updateOperatorMetadataURI","inputs":[{"name":"metadataURI","type":"string","internalType":"string"}],"outputs":[],"stateMutability":"nonpayable"},{"type":"event","name":"Initialized","inputs":[{"name":"version","type":"uint8","indexed":false,"internalType":"uint8"}],"anonymous":false},{"type":"event","name":"MinWithdrawalDelayBlocksSet","inputs":[{"name":"previousValue","type":"uint256","indexed":false,"internalType":"uint256"},{"name":"newValue","type":"uint256","indexed":false,"internalType":"uint256"}],"anonymous":false},{"type":"event","name":"OperatorDetailsModified","inputs":[{"name":"operator","type":"address","indexed":true,"internalType":"address"},{"name":"newOperatorDetails","type":"tuple","indexed":false,"internalType":"struct IDelegationManager.OperatorDetails","components":[{"name":"earningsReceiver","type":"address","internalType":"address"},{"name":"delegationApprover","type":"address","internalType":"address"},{"name":"stakerOptOutWindowBlocks","type":"uint32","internalType":"uint32"}]}],"anonymous":false},{"type":"event","name":"OperatorMetadataURIUpdated","inputs":[{"name":"operator","type":"address","indexed":true,"internalType":"address"},{"name":"metadataURI","type":"string","indexed":false,"internalType":"string"}],"anonymous":false},{"type":"event","name":"OperatorRegistered","inputs":[{"name":"operator","type":"address","indexed":true,"internalType":"address"},{"name":"operatorDetails","type":"tuple","indexed":false,"internalType":"struct IDelegationManager.OperatorDetails","components":[{"name":"earningsReceiver","type":"address","internalType":"address"},{"name":"delegationApprover","type":"address","internalType":"address"},{"name":"stakerOptOutWindowBlocks","type":"uint32","internalType":"uint32"}]}],"anonymous":false},{"type":"event","name":"OperatorSharesDecreased","inputs":[{"name":"operator","type":"address","indexed":true,"internalType":"address"},{"name":"staker","type":"address","indexed":false,"internalType":"address"},{"name":"strategy","type":"address","indexed":false,"internalType":"contract IStrategy"},{"name":"shares","type":"uint256","indexed":false,"internalType":"uint256"}],"anonymous":false},{"type":"event","name":"OperatorSharesIncreased","inputs":[{"name":"operator","type":"address","indexed":true,"internalType":"address"},{"name":"staker","type":"address","indexed":false,"internalType":"address"},{"name":"strategy","type":"address","indexed":false,"internalType":"contract IStrategy"},{"name":"shares","type":"uint256","indexed":false,"internalType":"uint256"}],"anonymous":false},{"type":"event","name":"OwnershipTransferred","inputs":[{"name":"previousOwner","type":"address","indexed":true,"internalType":"address"},{"name":"newOwner","type":"address","indexed":true,"internalType":"address"}],"anonymous":false},{"type":"event","name":"Paused","inputs":[{"name":"account","type":"address","indexed":true,"internalType":"address"},{"name":"newPausedStatus","type":"uint256","indexed":false,"internalType":"uint256"}],"anonymous":false},{"type":"event","name":"PauserRegistrySet","inputs":[{"name":"pauserRegistry","type":"address","indexed":false,"internalType":"contract IPauserRegistry"},{"name":"newPauserRegistry","type":"address","indexed":false,"internalType":"contract IPauserRegistry"}],"anonymous":false},{"type":"event","name":"StakerDelegated","inputs":[{"name":"staker","type":"address","indexed":true,"internalType":"address"},{"name":"operator","type":"address","indexed":true,"internalType":"address"}],"anonymous":false},{"type":"event","name":"StakerForceUndelegated","inputs":[{"name":"staker","type":"address","indexed":true,"internalType":"address"},{"name":"operator","type":"address","indexed":true,"internalType":"address"}],"anonymous":false},{"type":"event","name":"StakerUndelegated","inputs":[{"name":"staker","type":"address","indexed":true,"internalType":"address"},{"name":"operator","type":"address","indexed":true,"internalType":"address"}],"anonymous":false},{"type":"event","name":"StrategyWithdrawalDelayBlocksSet","inputs":[{"name":"strategy","type":"address","indexed":false,"internalType":"contract IStrategy"},{"name":"previousValue","type":"uint256","indexed":false,"internalType":"uint256"},{"name":"newValue","type":"uint256","indexed":false,"internalType":"uint256"}],"anonymous":false},{"type":"event","name":"Unpaused","inputs":[{"name":"account","type":"address","indexed":true,"internalType":"address"},{"name":"newPausedStatus","type":"uint256","indexed":false,"internalType":"uint256"}],"anonymous":false},{"type":"event","name":"WithdrawalCompleted","inputs":[{"name":"withdrawalRoot","type":"bytes32","indexed":false,"internalType":"bytes32"}],"anonymous":false},{"type":"event","name":"WithdrawalMigrated","inputs":[{"name":"oldWithdrawalRoot","type":"bytes32","indexed":false,"internalType":"bytes32"},{"name":"newWithdrawalRoot","type":"bytes32","indexed":false,"internalType":"bytes32"}],"anonymous":false},{"type":"event","name":"WithdrawalQueued","inputs":[{"name":"withdrawalRoot","type":"bytes32","indexed":false,"internalType":"bytes32"},{"name":"withdrawal","type":"tuple","indexed":false,"internalType":"struct IDelegationManager.Withdrawal","components":[{"name":"staker","type":"address","internalType":"address"},{"name":"delegatedTo","type":"address","internalType":"address"},{"name":"withdrawer","type":"address","internalType":"address"},{"name":"nonce","type":"uint256","internalType":"uint256"},{"name":"startBlock","type":"uint32","internalType":"uint32"},{"name":"strategies","type":"address[]","internalType":"contract IStrategy[]"},{"name":"shares","type":"uint256[]","internalType":"uint256[]"}]}],"anonymous":false}]"#,
    event_derives(serde::Deserialize, serde::Serialize)
);
