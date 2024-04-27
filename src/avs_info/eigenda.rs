use dialoguer::Input;
use ethers_core::types::{Address, H160, U256};
use ethers_core::utils::format_units;
use sys_info::{DiskInfo, MemInfo};

use crate::eigen::{delegation_manager, node_classes};
use crate::eigen::dgm_info::EigenStrategy;
use crate::eigen::node_classes::NodeClass;
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

    let passed = check_stake_and_system_requirements(&operator_address, network).await?;
    if !passed {
        let answer: String = Input::new()
            .with_prompt(
                "You do not meet the requirements to boot up EigenDA based on  your stake. Do it anyway? (y/n)",
            )
            .interact_text()
            .expect("Error reading key name");

        if answer.to_lowercase() != "y" {
            return Ok(());
        }
    }

    Ok(())
}

pub async fn check_stake_and_system_requirements(
    address: &str,
    network: Network,
) -> Result<bool, Box<dyn std::error::Error>> {
    let stake_map = delegation_manager::get_all_statregies_delegated_stake(address.to_string()).await?;

    let eth_stake: U256 = stake_map.get(&EigenStrategy::BeaconEth).expect("Amount should never be none, should always be 0").clone();
    let weth_stake: U256 = stake_map.get(&EigenStrategy::Weth).expect("Amount should never be none, should always be 0").clone();
    println!("Your stake in quorum 0 - ETH: {:?}", format_units(eth_stake, "ether").unwrap());
    println!("Your stake in quorum 1 - WETH: {:?}", format_units(weth_stake, "ether").unwrap());

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

    let passed = match network {
        Network::Mainnet => check_mainnet_stake_mins(eth_percentage, weth_percentage),
        Network::Holesky => check_testnet_stake_mins(eth_percentage, weth_percentage),
        Network::Local => todo!("Local network not supported yet"),
    }?;
    Ok(passed)
}

pub fn check_mainnet_stake_mins(eth_percentage: U256, weth_percentage: U256) -> Result<bool, Box<dyn std::error::Error>> {
    todo!("Mainnet stake minimums not implemented yet")
}

pub fn check_testnet_stake_mins(eth_percentage: U256, weth_percentage: U256) -> Result<bool, Box<dyn std::error::Error>> {
    let class = node_classes::get_node_class()?;
    match eth_percentage {
        x if x < U256::from(3) => {
            

        }
        x if x < U256::from(20) => {
            todo!();
        }
        x if x < U256::from(100) => {
            todo!();
        }
        x if x < U256::from(1000) => {
            todo!();
        }
        x if x > U256::from(2000) => {
            todo!();
        }
        _ => {}

    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use mockall::automock;
    use mockall::predicate::*;

    #[automock]
    pub trait Config {
        fn get_system_information(&self) -> Result<(u32, MemInfo, DiskInfo), Box<dyn std::error::Error>>;
    }

    // #[tokio::test]
    // async fn test_low_eth_weth_stakes() {

    //         let mem_info: MemInfo = MemInfo { total: 8589934, free: 0, avail: 0, buffers: 0, cached: 0, swap_total: 0, swap_free: 0 };
    //         let disk_info: DiskInfo = DiskInfo { total: 0, free: 0 };

    //         let mut mock = MockConfig::new();
    //         mock.expect_get_system_information()
    //             .times(1)
    //             .returning(move || Ok((2, MemInfo { total: 8589934, free: 0, avail: 0, buffers: 0, cached: 0, swap_total: 0, swap_free: 0 }, DiskInfo { total: 0, free: 0 })));
    //         //Low stake
    //         let lowstake = check_stake_and_system_requirements(
    //             "0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd",
    //             Network::Holesky,
    //         ).await.unwrap();

    //         // //Medium stake
    //         // let midstake = check_stake_and_system_requirements(
    //         //     Network::Mainnet,
    //         //     (U256::from(10 * (10 ^ 18)), U256::from(10 * (10 ^ 18))),
    //         // );

    //         // let highstake = check_stake_and_system_requirements(
    //         //     Network::Mainnet,
    //         //     (U256::from(10 * (10 ^ 18)), U256::from(10 * (10 ^ 18))),
    //         // );

    //     //    println!(lowstake);
    // }

    #[tokio::test]
    async fn test_stake_percentages() {
        let lowstake = check_stake_and_system_requirements(
                        "0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd",
                        Network::Holesky,
                    ).await.unwrap();
    }



}
