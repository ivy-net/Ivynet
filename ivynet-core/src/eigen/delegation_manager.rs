use super::dgm_info;
use crate::rpc_management;
use dgm_info::EigenStrategy;
use ethers_core::types::{Address, H160, U256};
use once_cell::sync::Lazy;
use std::collections::HashMap;

type DelegationManager = dgm_info::DelegationManagerAbi<rpc_management::Client>;

static DELEGATION_MANAGER: Lazy<DelegationManager> = Lazy::new(setup_delegation_manager_contract);
static STRATEGY_LIST: Lazy<Vec<EigenStrategy>> = Lazy::new(dgm_info::get_strategy_list);

fn setup_delegation_manager_contract() -> DelegationManager {
    let del_mgr_addr: Address =
        dgm_info::get_delegation_manager_address().parse().expect("Could not parse DelegationManager address");
    dgm_info::DelegationManagerAbi::new(del_mgr_addr.clone(), rpc_management::get_client())
}

pub async fn get_operator_details(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let operator_address = address.parse::<Address>()?;

    let status = DELEGATION_MANAGER.is_operator(operator_address).call().await?;
    println!("Is operator: {:?}", status);
    let details = DELEGATION_MANAGER.operator_details(operator_address).call().await?;
    println!("Operator details: {:?}", details);

    Ok(())
}

pub async fn get_staker_delegatable_shares(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let staker_address = address.parse::<Address>()?;
    let details = DELEGATION_MANAGER.get_delegatable_shares(staker_address).call().await?;
    println!("Staker delegatable shares: {:?}", details);
    Ok(())
}

// Function to get all strategies' delegated stake to an operator
pub async fn get_all_statregies_delegated_stake(
    address: String,
) -> Result<HashMap<EigenStrategy, U256>, Box<dyn std::error::Error>> {
    let operator_address = address.parse::<Address>()?;
    println!("Shares for operator: {:?}", operator_address);

    let mut strat_list: Vec<H160> = Vec::new();

    for i in 0..STRATEGY_LIST.len() {
        let str_strat: &str = STRATEGY_LIST[i].clone().into();
        let hex_strat = str_strat.parse::<Address>()?;
        strat_list.push(hex_strat)
    }

    let shares: Vec<U256> = DELEGATION_MANAGER.get_operator_shares(operator_address, strat_list).call().await?;

    let mut stake_map: HashMap<EigenStrategy, U256> = HashMap::new();
    for i in 0..STRATEGY_LIST.len() {
        stake_map.insert(STRATEGY_LIST[i], shares[i]);
    }

    Ok(stake_map)
}

pub async fn get_operator_status(address: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let operator_address = address.parse::<Address>()?;
    let status: bool = DELEGATION_MANAGER.is_operator(operator_address).call().await?;
    println!("Operator status: {:?}", status);

    Ok(status)
}
