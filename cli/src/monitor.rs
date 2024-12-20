use anyhow::anyhow;
use dialoguer::MultiSelect;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use ivynet_core::{
    config::DEFAULT_CONFIG_PATH,
    docker::dockerapi::DockerClient,
    grpc::{self, backend::backend_client::BackendClient, messages::Digests, tonic::Request},
    io::{read_toml, write_toml, IoError},
    node_type::NodeType,
    telemetry::{fetch_telemetry_from, listen, ConfiguredAvs},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::init::set_backend_connection;

const MONITOR_CONFIG_FILE: &str = "monitor-config.toml";

#[derive(Clone, Debug)]
struct PotentialAvs {
    pub container_name: String,
    pub image_hash: String,
    pub ports: Vec<u16>,
}

#[derive(thiserror::Error, Debug)]
pub enum MonitorConfigError {
    #[error(transparent)]
    ConfigIo(#[from] IoError),
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MonitorConfig {
    /// Configured AVSes to monitor
    pub configured_avses: Vec<ConfiguredAvs>,
}

impl MonitorConfig {
    pub fn load(path: PathBuf) -> Result<Self, MonitorConfigError> {
        let config: Self = read_toml(&path)?;
        Ok(config)
    }

    pub fn load_from_default_path() -> Result<Self, MonitorConfigError> {
        let config_path = DEFAULT_CONFIG_PATH.to_owned().join(MONITOR_CONFIG_FILE);
        // Previous impl built a bad path - let this error properly
        Self::load(config_path)
    }

    pub fn store(&self) -> Result<(), MonitorConfigError> {
        let config_path = DEFAULT_CONFIG_PATH.to_owned().join(MONITOR_CONFIG_FILE);
        write_toml(&config_path, self)?;
        Ok(())
    }
}

pub async fn start_monitor() -> Result<(), anyhow::Error> {
    let mut config = ivynet_core::config::IvyConfig::load_from_default_path()?;

    if config.identity_wallet().is_err() {
        set_backend_connection(&mut config).await?;
    }

    let monitor_config = MonitorConfig::load_from_default_path().unwrap_or_default();
    if monitor_config.configured_avses.is_empty() {
        return Err(anyhow!("No AVSes configured to monitor"));
    }

    // Validate uniqueness of assigned names
    let mut seen_names = std::collections::HashSet::new();
    for avs in &monitor_config.configured_avses {
        if !seen_names.insert(&avs.assigned_name) {
            return Err(anyhow!(
                "Duplicate AVS name found: {}. Each AVS must have a unique name.",
                avs.assigned_name
            ));
        }
    }

    let identity_wallet = config.identity_wallet()?;
    let machine_id = config.machine_id;
    let backend_url = config.get_server_url()?;
    let backend_ca = config.get_server_ca();
    let backend_ca = if backend_ca.is_empty() { None } else { Some(backend_ca) };

    let backend_client = BackendClient::new(
        grpc::client::create_channel(backend_url, backend_ca).await.expect("Cannot create channel"),
    );

    info!("Starting monitor listener...");
    listen(backend_client, machine_id, identity_wallet, &monitor_config.configured_avses).await?;
    Ok(())
}

/// Scan function to set up configured AVS cache file. Derives `NodeType` from the name on the
/// metrics port and node name from the container name list.
pub async fn scan() -> Result<(), anyhow::Error> {
    let config = ivynet_core::config::IvyConfig::load_from_default_path()?;
    let backend_url = config.get_server_url()?;
    let backend_ca = config.get_server_ca();
    let backend_ca = if backend_ca.is_empty() { None } else { Some(backend_ca) };

    let mut backend = BackendClient::new(
        grpc::client::create_channel(backend_url, backend_ca).await.expect("Cannot create channel"),
    );
    let docker = DockerClient::default();
    println!("Scanning for existing containers...");
    let images = docker.list_images().await;
    let potential_avses = docker
        .list_containers()
        .await
        .into_iter()
        .filter_map(|c| {
            if let (Some(names), Some(image_name)) = (c.names, c.image) {
                let mut ports = if let Some(ports) = c.ports {
                    ports.into_iter().filter_map(|p| p.public_port).collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                ports.sort();
                ports.dedup();
                if let Some(image_hash) = images.get(&image_name) {
                    return Some(PotentialAvs {
                        container_name: names.first().unwrap_or(&image_name).to_string(),
                        image_hash: image_hash.to_string(),
                        ports,
                    });
                }
            }
            None
        })
        .collect::<Vec<_>>();

    let mut monitor_config = MonitorConfig::load_from_default_path().unwrap_or_default();
    let mut avses = Vec::new();

    let configured_avs_names = monitor_config
        .configured_avses
        .iter()
        .map(|a| a.container_name.clone())
        .collect::<Vec<_>>();

    let digests = potential_avses
        .iter()
        .filter_map(|a| {
            if configured_avs_names.contains(&a.container_name) {
                None
            } else {
                Some(a.image_hash.clone())
            }
        })
        .collect::<Vec<_>>();

    let mut avs_hashes = HashMap::new();
    for ntype in
        backend.node_types(Request::new(Digests { digests })).await?.into_inner().node_types
    {
        avs_hashes.insert(ntype.digest, NodeType::from(ntype.node_type.as_str()));
    }

    for avs in &potential_avses {
        if !configured_avs_names.contains(&avs.container_name) {
            if let Some(avs_type) = avs_hashes.get(&avs.image_hash) {
                let mut metric_port = 0;

                for port in &avs.ports {
                    if let Ok(metrics) = fetch_telemetry_from(*port).await {
                        if !metrics.is_empty() {
                            // Checking performance score metrics to read a potential avs type

                            metric_port = *port;
                        }
                    }
                }
                avses.push(ConfiguredAvs {
                    assigned_name: String::new(),
                    container_name: avs.container_name.clone(),
                    avs_type: *avs_type,
                    metric_port,
                });
            }
        }
    }

    if avses.is_empty() {
        println!("No potential new AVSes found");
    } else {
        for idx in MultiSelect::new()
            .with_prompt("The following AVS types were found. Choose what AVSes to add with SPACE and accept the list with ENTER")
            .items(
                &avses
                    .iter()
                    .map(|a| format!("{} under container {}", a.avs_type, a.container_name))
                    .collect::<Vec<_>>(),
            )
            .interact()
            .expect("No items selected")
        {

            monitor_config.configured_avses.push(avses[idx].clone());
        }

        let mut seen_names: HashSet<String> =
            monitor_config.configured_avses.iter().map(|a| a.assigned_name.clone()).collect();
        for avs in &mut monitor_config.configured_avses {
            if avs.assigned_name.is_empty() {
                let mut assigned_name: String;
                loop {
                    assigned_name = dialoguer::Input::new()
                        .with_prompt(format!(
                            "Enter a name for this AVS that is Unique Per Machine: {}",
                            avs.container_name
                        ))
                        .interact_text()
                        .expect("Failed to get assigned name");

                    if seen_names.contains(&assigned_name) {
                        println!(
                            "Error: Name '{}' is already in use. Please choose a unique name.",
                            assigned_name
                        );
                        continue;
                    }

                    seen_names.insert(assigned_name.clone());
                    break;
                }
                avs.assigned_name = assigned_name;
            }
        }

        monitor_config.store()?;
        println!(
            "New setup stored with {} of avses configured",
            monitor_config.configured_avses.len()
        );
    }
    Ok(())
}
