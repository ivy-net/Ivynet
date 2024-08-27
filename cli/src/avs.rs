use bollard::{
    container::{ListContainersOptions, LogsOptions},
    Docker,
};
use dialoguer::Password;
//use dialoguer::Input;
use dirs::home_dir;
use futures_util::stream::StreamExt;
use ivynet_core::{
    avs::{build_avs_provider, commands::AvsCommands},
    config::IvyConfig,
    grpc::client::{create_channel, Source},
    wallet::IvyWallet,
};
use std::{
    error::Error as BoxError,
    fs,
    io::{self, BufRead},
    path::Path,
};
use tokio::time::{sleep, Duration};

use crate::{client::IvynetClient, error::Error};

pub async fn parse_avs_subcommands(subcmd: AvsCommands, config: &IvyConfig) -> Result<(), Error> {
    let sock = Source::Path(config.uds_dir());

    // Setup runs local, otherwise construct a client and continue.
    if let AvsCommands::Setup { ref avs, ref chain } = subcmd {
        let password: String = Password::new()
            .with_prompt("Input the password for your stored operator ECDSA keyfile")
            .interact()?;
        let wallet = IvyWallet::from_keystore(config.default_ecdsa_keyfile.clone(), &password)?;
        let avs =
            build_avs_provider(Some(avs), chain, config, Some(wallet), Some(password.clone()))
                .await?;
        avs.setup(config, Some(password)).await?;
        return Ok(());
    }

    let mut client = IvynetClient::from_channel(create_channel(sock, None).await?);
    match subcmd {
        AvsCommands::Info {} => {
            let response = client.avs_mut().avs_info().await?;
            println!("{:?}", response.into_inner());
        }
        // TODO: Fix timeout issue
        AvsCommands::Register {} => {
            client.avs_mut().opt_in().await?;
            println!("{}", get_register_logs());
        }
        AvsCommands::Unregister {} => {
            let response = client.avs_mut().opt_out().await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Start { avs, chain } => {
            client.avs_mut().start(avs, chain).await?;
            check_docker_logs_for_errors().await.expect("Couldn't obtain logs");
        }
        AvsCommands::Stop {} => {
            let response = client.avs_mut().stop().await?;

            println!("{:?}", response.into_inner());
        }
        AvsCommands::Select { avs, chain } => {
            let response = client.avs_mut().select_avs(avs, chain).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::CheckStakePercentage { .. } => todo!(),
        _ => unimplemented!("Command not implemented: {:?}", subcmd),
    }
    Ok(())
}

// TODO:
// Check error on following flows:
// - Start without setup (currently returns "no such file")
async fn check_docker_logs_for_errors() -> Result<(), Box<dyn BoxError>> {
    sleep(Duration::from_secs(1)).await;
    let logs = get_logs_from_all_error_containers().await?;
    process_logs_and_check_errors(&logs).await;
    Ok(())
}
pub async fn get_container_ids(docker: &Docker) -> Result<Vec<String>, Box<dyn BoxError>> {
    let options = Some(ListContainersOptions::<String> { all: false, ..Default::default() });

    let containers = docker.list_containers(options).await?;

    let container_ids: Vec<String> =
        containers.into_iter().map(|container| container.id.unwrap_or_default()).collect();

    Ok(container_ids)
}

pub async fn get_docker_error_logs(
    docker: &Docker,
    container_id: &str,
) -> Result<String, Box<dyn BoxError>> {
    let mut logs_stream = docker.logs(
        container_id,
        Some(LogsOptions {
            follow: false,
            stdout: true,
            stderr: true,
            timestamps: false,
            tail: "all",
            ..Default::default()
        }),
    );

    let mut error_logs = String::new();

    while let Some(log_result) = logs_stream.next().await {
        match log_result {
            Ok(log) => {
                let log_str: String = std::str::from_utf8(&log.into_bytes())?.to_string();
                if log_str.to_lowercase().contains("bls") {
                    error_logs.push_str(&log_str);
                }
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    Ok(error_logs)
}

pub async fn process_logs_and_check_errors(logs: &str) {
    for line in logs.lines() {
        if line.contains("application failed: could not read or decrypt the BLS private key: could not decrypt key with given password") {
            return println!("Error: Incorrect password for BLS key");
        } else if line.contains("application failed: could not read or decrypt the BLS private key: read /app/operator_keys/bls_key.json: is a directory") {
            return println!("Error: No BLS key found");
        }
    }
    println!("No critical errors found.");
}

pub async fn get_logs_from_all_error_containers() -> Result<String, Box<dyn BoxError>> {
    let docker = Docker::connect_with_local_defaults()?;

    let mut container_ids = get_container_ids(&docker).await?;
    while container_ids.is_empty() {
        container_ids = get_container_ids(&docker).await?;
    }

    let mut all_error_logs = String::new();
    for container_id in container_ids {
        let logs = get_docker_error_logs(&docker, &container_id).await?;
        if !logs.is_empty() {
            all_error_logs.push_str(&format!("Logs for container {}:\n", container_id));
            all_error_logs.push_str(&logs);
        }
    }

    Ok(all_error_logs)
}

fn get_register_logs() -> String {
    let file_path = match home_dir() {
        Some(mut path) => {
            path.push(".eigenlayer/eigenda/eigenda-operator-setup/holesky/script_output.log");
            path
        }
        None => return "Succesfully registration".to_string(),
    };

    if !Path::new(&file_path).exists() {
        return String::new();
    }

    let file = match fs::File::open(&file_path) {
        Ok(file) => file,
        Err(_) => return String::new(),
    };

    let reader = io::BufReader::new(file);
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                if line.contains("insufficient funds for transfer") {
                    return "Error: insufficient funds for transfer".to_string();
                } else if line.contains("failed to read or decrypt the BLS private key") {
                    if line.contains("could not decrypt key with given password") {
                        return "Error: Incorrect password given to BLS key".to_string();
                    } else if line.contains("is a directory") {
                        return "Error: No valid BLS key found".to_string();
                    }
                    return "Error: Non-correct BLS key".to_string();
                }
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
            }
        }
    }
    String::new()
}
