use ethers_core::k256;
use ethers_core::types::Address;
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::{LocalWallet, Wallet};
use std::convert::TryFrom;
use std::sync::Arc;

use super::eigen_info;
use crate::{config, keys};

type Client = SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>;
type DelegationManager = eigen_info::DelegationManager<Client>;

lazy_static::lazy_static! {
    static ref PROVIDER: Provider<Http> = connect_provider();
    static ref WALLET: LocalWallet = keys::connect_wallet();
    static ref CLIENT: Arc<Client> = Arc::new(SignerMiddleware::new(PROVIDER.clone(), WALLET.clone()));
    static ref DELEGATION_MANAGER: DelegationManager = setup_delegation_manager_contract();
}

fn connect_provider() -> Provider<Http> {
    let cfg = config::get_config();
    Provider::<Http>::try_from(&cfg.rpc_url).expect("Could not connect to provider")
}

pub fn setup_delegation_manager_contract() -> DelegationManager {
    let del_mgr_addr: Address = eigen_info::DELEGATION_MANAGER_ADDRESS
        .parse()
        .expect("Could not parse DelegationManager address");
    let arc_signer = Arc::new(SignerMiddleware::new(PROVIDER.clone(), WALLET.clone()));
    eigen_info::DelegationManager::new(del_mgr_addr.clone(), arc_signer)
}

pub async fn get_operator_details(address: String) -> Result<(), Box<dyn std::error::Error>> {
    let operator_address = address.parse::<Address>()?;

    let status = DELEGATION_MANAGER.is_operator(operator_address).call().await?;
    println!("Is operator: {:?}", status);
    let details = DELEGATION_MANAGER.operator_details(operator_address).call().await?;
    println!("Operator details: {:?}", details);

    Ok(())
}

pub async fn get_staker_delegatable_shares(address: String) -> Result<(), Box<dyn std::error::Error>> {
    let staker_address = address.parse::<Address>()?;
    let details = DELEGATION_MANAGER.get_delegatable_shares(staker_address).call().await?;
    println!("Staker delegatable shares: {:?}", details);

    Ok(())
}
