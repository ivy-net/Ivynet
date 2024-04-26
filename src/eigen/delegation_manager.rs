use std::collections::HashMap;

use ethers_core::types::{Address, H160, U256};
use ethers_core::utils::format_units;

use dgm_info::{EigenStrategy, STRATEGY_LIST};

use crate::rpc_management;

use super::dgm_info;

type DelegationManager = dgm_info::DelegationManagerAbi<rpc_management::Client>;

lazy_static::lazy_static! {
    static ref DELEGATION_MANAGER: DelegationManager = setup_delegation_manager_contract();
}

pub fn setup_delegation_manager_contract() -> DelegationManager {
    let del_mgr_addr: Address = dgm_info::DELEGATION_MANAGER_ADDRESS
        .parse()
        .expect("Could not parse DelegationManager address");
    dgm_info::DelegationManagerAbi::new(del_mgr_addr.clone(), rpc_management::CLIENT.clone())
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

    let shares: Vec<U256> = DELEGATION_MANAGER
        .get_operator_shares(operator_address, strat_list)
        .call()
        .await?;

    let mut stake_map: HashMap<EigenStrategy, U256> = HashMap::new();
    for i in 0..STRATEGY_LIST.len() {
        stake_map.insert(STRATEGY_LIST[i], shares[i]);
        println!(
            "Share Type: {:?}, Amount: {:?}",
            STRATEGY_LIST[i],
            format_units(shares[i], "ether").unwrap()
        );
    }

    Ok(stake_map)
}

pub async fn get_operator_status(address: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let operator_address = address.parse::<Address>()?;
    let status: bool = DELEGATION_MANAGER.is_operator(operator_address).call().await?;
    println!("Operator status: {:?}", status);

    Ok(status)
}
