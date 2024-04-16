use ethers_core::k256;
use ethers_core::types::{Address, H160, U256};
use ethers_core::utils::format_units;
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::{LocalWallet, Wallet};
use std::convert::TryFrom;
use std::sync::Arc;

use super::eigen_info::{self, STRATEGY_LIST};
use crate::rpc::eigen_info::EigenStrategy;
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

pub async fn get_all_statregies_delegated_stake(address: String) -> Result<(), Box<dyn std::error::Error>> {
    let operator_address = address.parse::<Address>()?;
    println!("Shares for operator: {:?}", operator_address);

    let mut strat_list: Vec<H160> = Vec::new();

    for i in 0..STRATEGY_LIST.len() {
        let str_strat: &str = STRATEGY_LIST[i].clone().into();
        let hex_strat = str_strat.parse::<Address>()?;
        strat_list.push(hex_strat)
    }

    let shares: Vec<U256> = DELEGATION_MANAGER.get_operator_shares(operator_address, strat_list).call().await?;

    for i in 0..STRATEGY_LIST.len() {
        println!(
            "Share Type: {:?}, Amount: {:?}",
            STRATEGY_LIST[i].clone(),
            format_units(shares[i], "ether").unwrap()
        );
    }

    Ok(())
}
