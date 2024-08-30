use ethers::utils::{Anvil, AnvilInstance};

pub fn attach_local_anvil() -> AnvilInstance {
    Anvil::new().fork("http://localhost:8545").spawn()
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
    async fn test_attach_local_anvil() {
        let anvil = attach_local_anvil();
        let key = anvil.keys().first().unwrap();
        let local_wallet = LocalWallet::from(key.clone());
        let http_provider = Provider::try_from(anvil.endpoint()).unwrap();
        let test_bal = http_provider.get_balance(local_wallet.address(), None).await.unwrap();
        assert_eq!(test_bal, U256::from(10000000000000000000000_u128));
    }
}
