use dialoguer::{Input, Password};
use ethers_core::{types::U256, utils::format_units};
use once_cell::sync::Lazy;
use rpc_management::Network;
use thiserror::Error;
use tokio::io::AsyncWriteExt;

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{copy, BufReader},
    path::{Path, PathBuf},
    process::Command,
};
use zip::read::ZipArchive;

use super::eigenda_info;
use crate::{
    config,
    eigen::{delegation_manager, dgm_info::EigenStrategy, node_classes, node_classes::NodeClass},
    keys, rpc_management,
};

pub static STAKE_REGISTRY: Lazy<eigenda_info::StakeRegistry> = Lazy::new(eigenda_info::setup_stake_registry);
pub static REGISTRY_COORDINATOR: Lazy<eigenda_info::RegistryCoordinator> =
    Lazy::new(eigenda_info::setup_registry_coordinator);
pub static REGISTRY_SIGNER: Lazy<eigenda_info::RegistryCoordinatorSigner> =
    Lazy::new(eigenda_info::setup_registry_coordinator_signer);
pub static QUORUMS: Lazy<HashMap<EigenStrategy, u8>> = Lazy::new(build_quorums);

#[derive(Error, Debug)]
pub enum EigenDAError {
    #[error("Boot script failed: {0}")]
    ScriptError(String),
}

pub async fn boot_eigenda() -> Result<(), Box<dyn std::error::Error>> {
    println!("Booting up AVS: EigenDA");
    println!("Checking system information and operator stake");
    let network: Network = rpc_management::get_network();
    let operator_address: String = keys::get_stored_public_key()?;

    let quorums_to_boot = check_stake_and_system_requirements(&operator_address, network).await?;
    println!("Quorums: {:?}", quorums_to_boot);

    //Individual quorums will need to be checked - cannot opt in to quorums they're already in
    let status = get_operator_status(&operator_address).await?;
    if status == 1 {
        //Check which quorums they're already in and register for the others they're eligible for
    } else {
        //Register operator for all quorums they're eligible for
    }

    let mut eigen_path = dirs::home_dir().expect("Could not get home directory");
    eigen_path.push(".eigenlayer/eigenda");
    match network {
        Network::Mainnet => eigen_path.push("mainnet"),
        Network::Holesky => eigen_path.push("holesky"),
        Network::Local => eigen_path.push("holesky"),
    }
    fs::create_dir_all(&eigen_path)?;

    download_operator_setup_files(eigen_path.clone()).await?;

    download_g1_g2(eigen_path.clone()).await?;

    build_env_file(network, eigen_path.clone()).await?;

    let quorums_converted: Vec<u8> = quorums_to_boot.iter().filter_map(|strat| QUORUMS.get(strat).cloned()).collect();
    let quorums_converted_str: String =
        quorums_converted.iter().map(|n| n.to_string()).collect::<Vec<String>>().join(",");
    println!("Quorums to boot: {}", quorums_converted_str);
    optin(quorums_converted_str, network, eigen_path)?;

    Ok(())
}

pub fn optin(quorums: String, network: Network, eigen_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let run_script_path = eigen_path.join("eigenda_operator_setup");
    let run_script_path = match network {
        Network::Mainnet => run_script_path.join("mainnet"),
        Network::Holesky => run_script_path.join("holesky"),
        Network::Local => run_script_path.join("holesky"),
    };

    let env_path = run_script_path.join(".env");
    let current_dir = std::env::current_dir()?;
    let current_env_path = current_dir.join(".env");

    // Copy .env file to current directory
    std::fs::copy(&env_path, &current_env_path)?;

    let ecdsa_password: String =
        Password::new().with_prompt("Input the password for your ECDSA key file for quorum opt-in").interact()?;

    let run_script_path = run_script_path.join("run.sh");
    let optin = Command::new("sh")
        .arg(run_script_path.clone())
        .arg("--operation-type")
        .arg("opt-in")
        .arg("--node-ecdsa-key-file-host")
        .arg(config::get_default_private_keyfile())
        .arg("--node-ecdsa-key-password")
        .arg(ecdsa_password)
        .arg("--quorums")
        .arg(quorums)
        .status()?;

    // Delete .env file from current directory
    std::fs::remove_file(current_env_path)?;

    if optin.success() {
        Ok(())
    } else {
        Err(Box::new(EigenDAError::ScriptError(optin.to_string())))
    }
}

fn edit_env_vars(filename: &str, env_values: HashMap<&str, &str>) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(filename)?;
    let new_contents = contents
        .lines()
        .map(|line| {
            let mut parts = line.splitn(2, '=');
            let key: &str = parts.next().unwrap();
            if let Some(value) = env_values.get(key) {
                format!("{}={}", key, value)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(filename, new_contents.as_bytes())?;
    Ok(())
}

pub async fn build_env_file(network: Network, eigen_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let run_script_path = eigen_path.join("eigenda_operator_setup");
    let run_script_path = match network {
        Network::Mainnet => run_script_path.join("mainnet"),
        Network::Holesky => run_script_path.join("holesky"),
        Network::Local => run_script_path.join("holesky"),
    };

    let mut set_vars: bool = false;

    let env_example_path = run_script_path.join(".env.example");
    let env_path = run_script_path.join(".env");
    if env_example_path.exists() && !env_path.exists() {
        std::fs::copy(env_example_path, env_path.clone())?;
        println!("Copied '.env.example' to '.env'.");
        set_vars = true;
    } else if !env_example_path.exists() {
        println!("The '.env.example' file does not exist.");
    } else {
        println!("The '.env' file already exists.");
        let reset_string: String = Input::new().with_prompt("Reset env file? (y/n)").interact_text()?;
        if reset_string == "y" {
            std::fs::remove_file(env_path.clone())?;
            std::fs::copy(env_example_path, env_path.clone())?;
            println!("Copied '.env.example' to '.env'.");
            set_vars = true;
        }
    }

    if set_vars {
        let mut env_values: HashMap<&str, &str> = HashMap::new();
        let node_hostname = reqwest::get("https://api.ipify.org").await?.text().await?;
        env_values.insert("NODE_HOSTNAME", &node_hostname);

        let rpc_url = &config::get_rpc_url(network)?;
        env_values.insert("NODE_CHAIN_RPC", rpc_url);

        let home_dir = dirs::home_dir().unwrap();
        let home_str = home_dir.to_str().expect("Could not get home directory");
        env_values.insert("USER_HOME", home_str);

        let bls_key_name: String = Input::new()
            .with_prompt(
                "Input the name of your BLS key file - looks in .eigenlayer folder (where eigen cli stores the key)",
            )
            .interact_text()?;

        let mut bls_json_file_location = dirs::home_dir().expect("Could not get home directory");
        bls_json_file_location.push(".eigenlayer/operator_keys");
        bls_json_file_location.push(bls_key_name);
        bls_json_file_location.set_extension("bls.key.json");
        println!("BLS key file location: {:?}", bls_json_file_location);
        env_values.insert(
            "NODE_BLS_KEY_FILE_HOST",
            bls_json_file_location.to_str().expect("Could not get BLS key file location"),
        );

        let bls_password: String =
            Password::new().with_prompt("Input the password for your BLS key file").interact()?;
        env_values.insert("NODE_BLS_KEY_PASSWORD", &bls_password);

        edit_env_vars(env_path.to_str().unwrap(), env_values)?;
    }

    Ok(())
}

pub async fn download_g1_g2(eigen_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let resources_dir = eigen_path.join("eigenda_operator_setup/resources");
    let g1_file_path = resources_dir.join("g1.point");
    let g2_file_path = resources_dir.join("g2.point.PowerOf2");
    if g1_file_path.exists() {
        println!("The 'g1.point' file already exists.");
    } else {
        println!("The 'g1.point' file does not exist, downloading appropriate file - 8.5GB!");
        // Download the "g1.point" file
        let g1_response = reqwest::get("https://srs-mainnet.s3.amazonaws.com/kzg/g1.point").await?;
        let bytes = g1_response.bytes().await?;
        let resources_dir = eigen_path.join("eigenda_operator_setup/resources");
        std::fs::create_dir_all(&resources_dir)?;
        let file_path = resources_dir.join("g1.point");
        let mut file = tokio::fs::File::create(&file_path).await?;
        file.write_all(&bytes).await?;
        println!("Downloaded g1.point");
    }

    if g2_file_path.exists() {
        println!("The 'g2.point.PowerOf2' file already exists.");
    } else {
        println!("The 'g2.point.PowerOf2' file does not exist, downloading appropriate file");
        //Download g2.point.powerOf2
        let g2_response = reqwest::get("https://srs-mainnet.s3.amazonaws.com/kzg/g2.point.powerOf2").await?;
        let bytes = g2_response.bytes().await?;
        let resources_dir = eigen_path.join("eigenda_operator_setup/resources");
        std::fs::create_dir_all(&resources_dir)?;
        let file_path = resources_dir.join("g2.point.PowerOf2");
        let mut file = tokio::fs::File::create(&file_path).await?;
        file.write_all(&bytes).await?;
        println!("Downloaded g2.point");
    }

    Ok(())
}

//Whole function needs to be cleaned up
pub async fn download_operator_setup_files(eigen_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut setup = false;
    let operator_setup_path = eigen_path.join("eigenda_operator_setup");
    if operator_setup_path.exists() {
        let reset_string: String = Input::new()
            .with_prompt("The 'extracted_files' directory already exists. Redownload? (y/n)")
            .interact_text()?;

        if reset_string == "y" {
            setup = true;
            fs::remove_dir_all(operator_setup_path)?;
        }
    } else {
        println!("The 'extracted_files' directory does not exist, downloading appropriate files");
        setup = true;
    }

    if setup {
        let repo_url = "https://github.com/ivy-net/eigenda-operator-setup/archive/refs/heads/master.zip";
        let response = reqwest::get(repo_url).await?;

        let mut dest = {
            let fname = response
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { None })
                .unwrap_or("eigenda_operator_setup.zip");

            File::create(fname)?
        };
        let bytes = response.bytes().await?;
        std::io::copy(&mut bytes.as_ref(), &mut dest)?;

        let reader = BufReader::new(File::open("eigenda_operator_setup.zip")?);
        let mut archive = ZipArchive::new(reader)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = eigen_path.join("setup_files").join(file.name());

            if (&*file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = File::create(&outpath)?;
                copy(&mut file, &mut outfile)?;
            }
        }

        let extracted_files_dir = eigen_path.join("setup_files");
        let first_dir = std::fs::read_dir(&extracted_files_dir)?
            .filter_map(Result::ok)
            .find(|entry| entry.file_type().unwrap().is_dir());
        if let Some(first_dir) = first_dir {
            let old_folder_path = first_dir.path();
            let new_folder_path = eigen_path.join("eigenda_operator_setup");
            std::fs::rename(&old_folder_path, &new_folder_path)?;
        }

        // Delete the "extracted_files" directory
        if extracted_files_dir.exists() {
            std::fs::remove_dir_all(extracted_files_dir)?;
        }

        // Delete the "eigenda_operator_setup.zip" file
        let zip_file_path = Path::new("eigenda_operator_setup.zip");
        if zip_file_path.exists() {
            std::fs::remove_file(zip_file_path)?;
        }
    }

    Ok(())
}

pub async fn get_operator_status(addr: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let operator_details = REGISTRY_COORDINATOR.get_operator(addr.parse()?).call().await?;
    // println!("Operator status: {:?}", operator_details.status);
    Ok(operator_details.status)
}

pub async fn check_stake_and_system_requirements(
    address: &str,
    network: Network,
) -> Result<Vec<EigenStrategy>, Box<dyn std::error::Error>> {
    let stake_map = delegation_manager::get_all_statregies_delegated_stake(address.to_string()).await?;
    println!("You are on network: {:?}", network);

    let bandwidth: u32 = Input::new().with_prompt("Input your bandwidth in mbps").interact_text()?;

    let mut quorums_to_boot: Vec<EigenStrategy> = Vec::new();
    for (strat, num) in QUORUMS.iter() {
        let quorum_stake: U256 = *stake_map.get(strat).expect("Amount should never be none, should always be 0");

        println!("Your stake in quorum {:?}: {:?}", strat, format_units(quorum_stake, "ether").unwrap());

        let quorum_total = STAKE_REGISTRY.get_current_total_stake(*num).call().await?;
        println!("Total stake in quorum 0 - {:?}: {:?}", strat, format_units(quorum_total, "ether").unwrap());

        // TODO: Check if the address is already an operator to get their appropriate percentage
        //For now, just assume they are not
        // let already_operator = STAKE_REGISTRY.is_operator(H160::from_str(address)?).call().await?;

        let quorum_percentage = quorum_stake * 10000 / (quorum_stake + quorum_total);
        println!("After registering, you would have {:?}/10000 of quorum {:?}", quorum_percentage, strat);

        let passed_mins = check_system_mins(quorum_percentage, bandwidth)?;
        match network {
            Network::Mainnet => {
                let stake_min: U256 = U256::from(96 * (10 ^ 18));
                if quorum_stake > stake_min && passed_mins {
                    quorums_to_boot.push(*strat);
                } else {
                    println!("You do not meet the requirements for quorum {:?}", strat);
                }
            }
            Network::Holesky => {
                let stake_min: U256 = U256::from(32 * (10 ^ 18));
                if quorum_stake > stake_min && passed_mins {
                    quorums_to_boot.push(*strat);
                } else {
                    println!("You do not meet the requirements for quorum {:?}", strat);
                }
            }
            Network::Local => {
                //If its local, presumably you want to try against a forked network and do the calls anyway
                quorums_to_boot.push(*strat)
            }
        }
    }

    Ok(quorums_to_boot)
}

fn check_system_mins(quorum_percentage: U256, bandwidth: u32) -> Result<bool, Box<dyn std::error::Error>> {
    let (_, _, disk_info) = config::get_system_information()?;
    let class = node_classes::get_node_class()?;

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

pub fn build_quorums() -> HashMap<EigenStrategy, u8> {
    let mut quorums: HashMap<EigenStrategy, u8> = HashMap::new();
    quorums.insert(EigenStrategy::BeaconEth, 0);
    quorums.insert(EigenStrategy::Weth, 1);
    quorums
}
