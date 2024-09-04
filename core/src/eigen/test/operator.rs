use crate::eigen::contracts::{delegation_manager, DelegationManagerAbi};
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Chain, H160};
use ethers::utils::AnvilInstance;
use std::sync::Arc;

use ethers::middleware::SignerMiddleware;

use crate::{
    eigen::contracts::OperatorDetails,
    test::local_anvil::{fork_holesky_anvil, fork_local_anvil, fork_mainnet_anvil},
};

use serial_test::serial;

#[allow(dead_code)]
/// For some reason, creating a signer with this function fails during testing, with an error
/// returning on the unwrap of the transation. No idea why. Function left in for curiosity's sake.
fn signer_fixture(anvil: AnvilInstance) -> Arc<SignerMiddleware<Provider<Http>, LocalWallet>> {
    let operator_key = anvil.keys()[9].clone();
    let operator = LocalWallet::from(operator_key.clone()).with_chain_id(anvil.chain_id());
    println!("CHAIN ID: {}", anvil.chain_id());
    println!("ENDPOINT: {}", anvil.endpoint());
    let provider = Provider::try_from(anvil.endpoint()).unwrap();

    Arc::new(SignerMiddleware::new(provider, operator))
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
        let anvil = fork_local_anvil();

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
        let anvil = fork_mainnet_anvil();
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
    use super::*;
    #[tokio::test]
    /// Test that an operator can register on the Eigenlayer network.
    async fn test_operator_can_register() {
        let anvil = fork_holesky_anvil();
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
}
