use ethers::{
    providers::{Middleware, Provider},
    utils::{Anvil, AnvilInstance},
};

use crate::{
    config::IvyConfig,
    eigen::test::common::{Eigenlayer, LOCAL_DEPLOYMENT_DEFAULT_PATH},
};

// Network forked tests currently have a hard dependency on correctly populated IvyConfig

/// Requres that a local anvil node be running on port 8545. See `avss/README.md for instructions.`
pub fn fork_local_anvil() -> AnvilInstance {
    let eigenlayer = Eigenlayer::load(LOCAL_DEPLOYMENT_DEFAULT_PATH.clone()).unwrap();
    eigenlayer.to_env();
    Anvil::new().fork("http://localhost:8545").spawn()
}

/// Fork mainnet anvil 50 blocks behind chain head.
pub async fn fork_mainnet_anvil() -> AnvilInstance {
    let config = IvyConfig::load_from_default_path().unwrap();
    let provider = Provider::try_from(&config.mainnet_rpc_url).unwrap();
    let chain_head = provider.get_block_number().await.expect("Could not get mainnet block number");
    Anvil::new().fork(config.mainnet_rpc_url).fork_block_number(chain_head.as_u64() - 50).spawn()
}

/// Fork holesky anvil 50 blocks behind chain head.
pub async fn fork_holesky_anvil() -> AnvilInstance {
    let config = IvyConfig::load_from_default_path().unwrap();
    let provider = Provider::try_from(&config.holesky_rpc_url).unwrap();
    let chain_head = provider.get_block_number().await.expect("Could not get mainnet block number");
    Anvil::new().fork(config.holesky_rpc_url).fork_block_number(chain_head.as_u64() - 50).spawn()
}

#[cfg(test)]
mod tests {
    use ethers::{
        providers::{Middleware, Provider},
        signers::{LocalWallet, Signer},
        types::U256,
    };

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_attach_local_anvil() {
        let anvil = fork_local_anvil();
        let key = anvil.keys().first().unwrap();
        let local_wallet = LocalWallet::from(key.clone());
        let http_provider = Provider::try_from(anvil.endpoint()).unwrap();
        println!("Local wallet address: {}", local_wallet.address());
        let test_bal = http_provider.get_balance(local_wallet.address(), None).await.unwrap();
        assert_eq!(test_bal, U256::from(10000000000000000000000_u128));
    }
}
