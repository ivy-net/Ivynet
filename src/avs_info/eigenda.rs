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

    //TODO: BOOT  THESE QUORUMS

    Ok(())
}

pub async fn check_stake_and_system_requirements(
    address: &str,
    network: Network,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let stake_map = delegation_manager::get_all_statregies_delegated_stake(address.to_string()).await?;

    let eth_stake: U256 = stake_map
        .get(&EigenStrategy::BeaconEth)
        .expect("Amount should never be none, should always be 0")
        .clone();
    let weth_stake: U256 = stake_map
        .get(&EigenStrategy::Weth)
        .expect("Amount should never be none, should always be 0")
        .clone();
    println!(
        "Your stake in quorum 0 - ETH: {:?}",
        format_units(eth_stake, "ether").unwrap()
    );
    println!(
        "Your stake in quorum 1 - WETH: {:?}",
        format_units(weth_stake, "ether").unwrap()
    );

    println!("You are on network: {:?}", network);

    let eth_total = STAKE_REGISTRY.get_current_total_stake(0).call().await?;
    let weth_total = STAKE_REGISTRY.get_current_total_stake(1).call().await?;

    // TODO: Check if the address is already an operator to get their appropriate percentage
    //For now, just assume they are not
    // let already_operator = STAKE_REGISTRY.is_operator(H160::from_str(address)?).call().await?;

    println!("Total stake in quorum 0 - ETH: {:?}", eth_total);
    println!("Total stake in quorum 1 - WETH: {:?}", weth_total);

    let eth_percentage = eth_stake * 10000 / (eth_stake + eth_total);
    let weth_percentage = weth_stake * 10000 / (weth_stake + weth_total);

    println!(
        "After registering, you would have {:?}/10000 of quorum 0 - ETH",
        eth_percentage
    );
    println!(
        "After registering, you would have {:?}/10000 of quorum 1 - WETH",
        weth_percentage
    );

    let quorums_passed = check_stake_mins(vec![eth_percentage, weth_percentage])?;

    let mut quorums: Vec<usize> = Vec::new();
    for (i, quorum_passed) in quorums_passed.iter().enumerate() {
        if !quorum_passed {
            println!("You do not meet the requirements for quorum {}", i);
        } else {
            quorums.push(i);
        }
    }

    Ok(quorums)
}

fn check_stake_mins(quorum_percentages: Vec<U256>) -> Result<Vec<bool>, Box<dyn std::error::Error>> {
    let (_, _, disk_info) = config::get_system_information()?;
    let class = node_classes::get_node_class()?;
    let bandwidth: u32 = Input::new()
        .with_prompt("Input your bandwidth in mbps:")
        .interact_text()
        .expect("Error reading bandwidth");

    let mut acceptability: Vec<bool> = Vec::new();
    for quorum_percentage in quorum_percentages {
        let mut acceptable: bool = true;
        match quorum_percentage {
            x if x < U256::from(3) => {
                if class < NodeClass::LRG || bandwidth < 1 || disk_info < 20000000000 {
                    acceptable = false
                }
            }
            x if x < U256::from(20) => {
                if class < NodeClass::LRG || bandwidth < 1 || disk_info < 150000000000 {
                    acceptable = false
                }
            }
            x if x < U256::from(100) => {
                if class < NodeClass::LRG || bandwidth < 1 || disk_info < 750000000000 {
                    acceptable = false
                }
            }
            x if x < U256::from(1000) => {
                if class < NodeClass::LRG || bandwidth < 1 || disk_info < 4000000000000 {
                    acceptable = false
                }
            }
            x if x > U256::from(2000) => {
                if class < NodeClass::LRG || bandwidth < 1 || disk_info < 8000000000000 {
                    acceptable = false
                }
            }
            _ => acceptable = false,
        }
        acceptability.push(acceptable);
    }
    Ok(acceptability)
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
