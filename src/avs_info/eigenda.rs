use dialoguer::Input;
use ethers_core::types::{Address, U256};
use ethers_core::utils::format_units;
use rpc_management::Network;

use super::eigenda_info;
use crate::eigen::dgm_info::EigenStrategy;
use crate::eigen::node_classes::NodeClass;
use crate::eigen::{delegation_manager, node_classes};
use crate::{config, keys, rpc_management};

type StakeRegistry = eigenda_info::EigendaStakeRegistryAbi<rpc_management::Client>;

lazy_static::lazy_static! {
    static ref STAKE_REGISTRY: StakeRegistry = setup_stake_registry();
    static ref QUORUMS: Vec<(EigenStrategy, u8)> = vec![(EigenStrategy::BeaconEth, 0), (EigenStrategy::Weth, 1)];
}

pub fn setup_stake_registry() -> StakeRegistry {
    let stake_reg_addr: Address = eigenda_info::STAKE_REGISTRY_ADDRESS
        .parse()
        .expect("Could not parse DelegationManager address");
    eigenda_info::EigendaStakeRegistryAbi::new(stake_reg_addr.clone(), rpc_management::CLIENT.clone())
}

pub async fn boot_eigenda() -> Result<(), Box<dyn std::error::Error>> {
    println!("Booting up AVS: EigenDA");
    println!("Checking system information and operator stake");
    let network: Network = rpc_management::get_network();
    let operator_address: String = keys::get_stored_public_key()?;

    let quorums_to_boot = check_stake_and_system_requirements(&operator_address, network).await?;

    println!("Quorums: {:?}", quorums_to_boot);


    //TODO: BOOT  THESE QUORUMS

    Ok(())
}

pub async fn check_stake_and_system_requirements(
    address: &str,
    network: Network,
) -> Result<Vec<EigenStrategy>, Box<dyn std::error::Error>> {
    let stake_map = delegation_manager::get_all_statregies_delegated_stake(address.to_string()).await?;
    println!("You are on network: {:?}", network);

    let stake_min: U256 = U256::from(96 * 10^18);

    let mut quorums_to_boot: Vec<EigenStrategy> = Vec::new();
    for (strat, num) in QUORUMS.iter() {
        let quorum_stake: U256 = stake_map
            .get(strat)
            .expect("Amount should never be none, should always be 0")
            .clone();

        println!(
            "Your stake in quorum 0 - {:?}: {:?}",
            strat,
            format_units(quorum_stake, "ether").unwrap()
        );

        let quorum_total = STAKE_REGISTRY.get_current_total_stake(num.clone()).call().await?;
        println!(
            "Total stake in quorum 0 - {:?}: {:?}",
            strat,
            format_units(quorum_total, "ether").unwrap()
        );

        // TODO: Check if the address is already an operator to get their appropriate percentage
        //For now, just assume they are not
        // let already_operator = STAKE_REGISTRY.is_operator(H160::from_str(address)?).call().await?;

        let quorum_percentage = quorum_stake * 10000 / (quorum_stake + quorum_total);
        println!(
            "After registering, you would have {:?}/10000 of quorum 0 - {:?}",
            quorum_percentage, strat
        );
        if quorum_stake > stake_min && check_system_mins(quorum_percentage)? {
            quorums_to_boot.push(strat.clone());
        }else {
            println!("You do not meet the requirements for quorum {:?}", strat);
        }
    }

    Ok(quorums_to_boot)
}

fn check_system_mins(quorum_percentage: U256) -> Result<bool, Box<dyn std::error::Error>> {
    let (_, _, disk_info) = config::get_system_information()?;
    let class = node_classes::get_node_class()?;
    let bandwidth: u32 = Input::new()
        .with_prompt("Input your bandwidth in mbps")
        .interact_text()
        .expect("Error reading bandwidth");

    let mut acceptable: bool = false;
    match quorum_percentage {
        x if x < U256::from(3) => {
            if class >= NodeClass::LRG || bandwidth >= 1 || disk_info >= 20000000000 {
                acceptable = true
            }
        }
        x if x < U256::from(20) => {
            if class >= NodeClass::XL || bandwidth >= 1 || disk_info >= 150000000000 {
                acceptable = true
            }
        }
        x if x < U256::from(100) => {
            if class >= NodeClass::FOURXL || bandwidth >= 3 || disk_info >= 750000000000 {
                acceptable = true
            }
        }
        x if x < U256::from(1000) => {
            if class >= NodeClass::FOURXL || bandwidth >= 25 || disk_info >= 4000000000000 {
                acceptable = true
            }
        }
        x if x > U256::from(2000) => {
            if class >= NodeClass::FOURXL || bandwidth >= 50 || disk_info >= 8000000000000 {
                acceptable = true
            }
        }
        _ => {}
    }
    Ok(acceptable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stake_percentages() {
        let lowstake =
            check_stake_and_system_requirements("0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd", Network::Holesky)
                .await
                .unwrap();
    }
}
