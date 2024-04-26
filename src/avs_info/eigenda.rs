use ethers_core::types::{Address, H160, U256};
use sys_info::{DiskInfo, MemInfo};

use crate::eigen::delegation_manager;
use crate::eigen::dgm_info::EigenStrategy;
use crate::errors::AVSError;
use crate::{config, keys, rpc_management};
use rpc_management::Network;

use super::eigenda_info;

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
    let stake_map = delegation_manager::get_all_statregies_delegated_stake(operator_address).await?;

    let eth_weth = (
        stake_map.get(&EigenStrategy::BeaconEth),
        stake_map.get(&EigenStrategy::Weth),
    );
    match eth_weth {
        (None, None) => {
            println!("Error: Operator has no stake in native ETH or WETH");
            return Err(Box::new(AVSError::NoStake));
        }
        _ => {}
    }

    check_stake_and_system_requirements(network, (eth_weth.0.unwrap().clone(), eth_weth.1.unwrap().clone())).await?;

    Ok(())
}

pub async fn check_stake_and_system_requirements(
    network: Network,
    eth_weth: (U256, U256),
) -> Result<(), Box<dyn std::error::Error>> {
    let (cpus, mem_info, disk_info) = config::get_system_information()?;
    println!("You are on network: {:?}", network);

    let eth_total = STAKE_REGISTRY.get_current_total_stake(0).call().await?;
    let weth_total = STAKE_REGISTRY.get_current_total_stake(1).call().await?;

    let eth_percentage = eth_weth.0 / (eth_weth.0 + eth_total);
    let weth_percentage = eth_weth.1 / (eth_weth.1 + weth_total);

    println!(
        "After registering, you would have {:?} percentage of quorum 0 - ETH",
        eth_percentage
    );
    println!(
        "After registering, you would have {:?} percentage of quorum 1 - WETH",
        weth_percentage
    );

    match network {
        Network::Mainnet => check_mainnet_stake_mins(eth_weth),
        Network::Testnet => check_testnet_stake_mins(eth_weth),
        Network::Local => todo!(),
    };
    Ok(())
}

pub fn check_mainnet_stake_mins(eth_weth: (U256, U256)) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn check_testnet_stake_mins(eth_weth: (U256, U256)) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}
