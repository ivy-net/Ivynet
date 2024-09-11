use crate::eigen::contracts::{
    delegation_manager, strategy_manager, DelegationManagerAbi, StrategyManagerAbi,
};
use crate::eigen::strategy::holesky::HoleskyLstStrategies;
use crate::test::contracts;
use crate::{
    eigen::contracts::OperatorDetails,
    test::local_anvil::{fork_holesky_anvil, fork_local_anvil, fork_mainnet_anvil},
};
use ethers::middleware::SignerMiddleware;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Chain, H160, U256};
use ethers::utils::AnvilInstance;
use std::sync::Arc;

type SignerClient = SignerMiddleware<Provider<Http>, LocalWallet>;

#[allow(dead_code)]
/// For some reason, creating a signer with this function fails during testing, with an error
/// returning on the unwrap of the transation. No idea why. Function left in for curiosity's sake.
fn signer_fixture(anvil: AnvilInstance) -> SignerClient {
    let operator_key = anvil.keys()[9].clone();
    let operator = LocalWallet::from(operator_key.clone()).with_chain_id(anvil.chain_id());
    println!("CHAIN ID: {}", anvil.chain_id());
    println!("ENDPOINT: {}", anvil.endpoint());
    let provider = Provider::try_from(anvil.endpoint()).unwrap();
    SignerMiddleware::new(provider, operator)
}

// https://testnet.ankr.com/staking/stake/ethereum/ for the ankr testnet frontend staking page
async fn enter_aeth_staking(amount: U256, signer: Arc<SignerClient>) {
    let ankr_steth = contracts::AnkrStethAbi::new(
        contracts::ankr_staked_eth(Chain::try_from(signer.get_chainid().await.unwrap()).unwrap()),
        signer,
    );
    let tx = ankr_steth.stake_and_claim_aeth_c().value(amount);
    let receipt = tx.send().await.unwrap().await.unwrap();
    // println!("{:#?}", receipt);
    assert!(receipt.is_some());
}

mod localhost {
    use super::*;
    #[tokio::test]
    /// Test that an operator can register on the Eigenlayer network.
    async fn test_operator_can_register() {
        let anvil = fork_local_anvil();
        let operator_key = anvil.keys()[9].clone();
        let operator = LocalWallet::from(operator_key.clone()).with_chain_id(anvil.chain_id());
        let provider = Provider::try_from(anvil.endpoint()).unwrap();

        let signer = Arc::new(SignerMiddleware::new(provider, operator));

        let delegation_manager =
            DelegationManagerAbi::new(delegation_manager(Chain::AnvilHardhat), signer);

        let operator_details = OperatorDetails {
            deprecated_earnings_receiver: H160::random(),
            delegation_approver: H160::zero(),
            staker_opt_out_window_blocks: 0,
        };
        let metadata_uri = "test.lol".to_string();

        let tx = delegation_manager.register_as_operator(operator_details, metadata_uri);
        let receipt = tx.send().await.unwrap().await.unwrap();
        assert!(receipt.is_some());
    }
}

mod mainnet {

    use super::*;
    #[tokio::test]
    /// Test that an operator can register on the Eigenlayer network.
    async fn test_operator_can_register() {
        let anvil = fork_mainnet_anvil().await;
        let operator_key = anvil.keys()[9].clone();
        let operator = LocalWallet::from(operator_key.clone()).with_chain_id(anvil.chain_id());
        let provider = Provider::try_from(anvil.endpoint()).unwrap();

        let signer = Arc::new(SignerMiddleware::new(provider, operator));

        let delegation_manager =
            DelegationManagerAbi::new(delegation_manager(Chain::Mainnet), signer);

        let operator_details = OperatorDetails {
            deprecated_earnings_receiver: H160::random(),
            delegation_approver: H160::zero(),
            staker_opt_out_window_blocks: 0,
        };
        let metadata_uri = "test.lol".to_string();

        let tx = delegation_manager.register_as_operator(operator_details, metadata_uri);
        let receipt = tx.send().await.unwrap().await.unwrap();
        assert!(receipt.is_some());
    }
}

mod holesky {
    use crate::eigen::strategy::StrategyList;

    use super::*;
    #[tokio::test]
    /// Test that an operator can register on the Eigenlayer network.
    async fn test_operator_can_register() {
        let anvil = fork_holesky_anvil().await;
        let operator_key = anvil.keys()[9].clone();
        let operator = LocalWallet::from(operator_key.clone()).with_chain_id(anvil.chain_id());
        let provider = Provider::try_from(anvil.endpoint()).unwrap();

        let signer = Arc::new(SignerMiddleware::new(provider, operator));

        let delegation_manager =
            DelegationManagerAbi::new(delegation_manager(Chain::Holesky), signer);

        let operator_details = OperatorDetails {
            deprecated_earnings_receiver: H160::random(),
            delegation_approver: H160::zero(),
            staker_opt_out_window_blocks: 0,
        };
        let metadata_uri = "test.lol".to_string();

        let tx = delegation_manager.register_as_operator(operator_details, metadata_uri);
        let receipt = tx.send().await.unwrap().await.unwrap();
        assert!(receipt.is_some());
    }
    #[tokio::test]
    async fn test_operator_has_stake() {
        let anvil = fork_holesky_anvil().await;
        let operator_key = anvil.keys()[9].clone();
        let operator = LocalWallet::from(operator_key.clone()).with_chain_id(anvil.chain_id());
        let provider = Provider::try_from(anvil.endpoint()).unwrap();

        let signer = Arc::new(SignerMiddleware::new(provider.clone(), operator));

        // Enter staking
        let one_ether = U256::from(1000000000000000000_u128);
        let amount = one_ether * U256::from(33_u64);
        let user_bal = provider.get_balance(signer.address(), None).await.unwrap();
        println!("User balance: {}", user_bal);
        enter_aeth_staking(amount, signer.clone()).await;

        let res: serde_json::Value = provider.request("anvil_mine", [1]).await.unwrap();

        let user_bal = provider.get_balance(signer.address(), None).await.unwrap();
        println!("User balance: {}", user_bal);

        let aeth_token =
            contracts::ERC20::new(HoleskyLstStrategies::Ankreth.token(), signer.clone());
        println!("Signer address: {}", signer.address());
        // let balance = aeth_token.balance_of(signer.address()).await.unwrap();
        // assert_eq!(balance, amount);

        let tx = aeth_token.approve(strategy_manager(Chain::Holesky), amount);
        let receipt = tx.send().await.unwrap().await.unwrap();
        assert!(receipt.is_some());

        let strategy_manager =
            StrategyManagerAbi::new(strategy_manager(Chain::Holesky), signer.clone());
        let strategy = HoleskyLstStrategies::Ankreth;
        let tx =
            strategy_manager.deposit_into_strategy(strategy.address(), strategy.token(), amount);
        let receipt = tx.send().await.unwrap().await.unwrap();
        assert!(receipt.is_some());

        let delegation_manager =
            DelegationManagerAbi::new(delegation_manager(Chain::Holesky), signer.clone());

        let stake =
            delegation_manager.operator_shares(signer.address(), strategy.address()).await.unwrap();
        println!("Stake: {}", stake);
        assert_eq!(stake, amount);
    }
}
