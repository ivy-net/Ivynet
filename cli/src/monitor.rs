use anyhow::anyhow;
use dialoguer::{Input, MultiSelect, Select};
use ivynet_docker::dockerapi::{DockerApi, DockerClient};
use ivynet_grpc::{
    self,
    backend::backend_client::BackendClient,
    client::create_channel,
    messages::{NodeTypeQueries, NodeTypeQuery, SignedNameChange},
    tonic::{transport::Channel, Request},
};
use ivynet_io::{read_toml, write_toml, IoError};
use ivynet_signer::sign_utils::sign_name_change;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use tracing::{debug, info};

use crate::{
    config::{IvyConfig, DEFAULT_CONFIG_PATH},
    init::set_backend_connection,
    telemetry::{listen, metrics_listener::fetch_telemetry_from, ConfiguredAvs},
};

const MONITOR_CONFIG_FILE: &str = "monitor-config.toml";

#[derive(Clone, Debug)]
struct PotentialAvs {
    pub container_name: String,
    pub image_name: String,
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

    pub fn change_avs_name(
        &mut self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), MonitorConfigError> {
        self.configured_avses.iter_mut().for_each(|avs| {
            if avs.assigned_name == old_name {
                avs.assigned_name = new_name.to_string();
            }
        });
        self.store()
    }
}

pub async fn rename_node(
    config: &IvyConfig,
    old_name: Option<String>,
    new_name: Option<String>,
) -> Result<(), anyhow::Error> {
    let mut monitor_config = MonitorConfig::load_from_default_path()?;

    let old = match old_name {
        Some(old_name) => old_name,
        None => {
            let configured_avs = &monitor_config
                .configured_avses
                .iter()
                .map(|a| a.assigned_name.clone())
                .collect::<Vec<_>>();
            let old_name = Select::new()
                .with_prompt("Select the old avs of the node to rename")
                .items(configured_avs)
                .default(0)
                .interact()
                .map_err(|e| anyhow!("Failed to get input: {}", e))?;
            configured_avs[old_name].clone()
        }
    };

    let new = match new_name {
        Some(new_name) => new_name,
        None => Input::new()
            .with_prompt("Enter the new name for the node")
            .interact_text()
            .map_err(|e| anyhow!("Failed to get input: {}", e))?,
    };

    let signature = sign_name_change(&old, &new, &config.identity_wallet()?)?;

    let machine_id = config.machine_id;
    let backend_url = config.get_server_url()?;
    let backend_ca = config.get_server_ca();
    let backend_ca = if backend_ca.is_empty() { None } else { Some(backend_ca) };

    let mut backend_client = BackendClient::new(
        create_channel(backend_url, backend_ca).await.expect("Cannot create channel"),
    );

    let name_change_request = Request::new(SignedNameChange {
        signature: signature.into(),
        machine_id: machine_id.into(),
        old_name: old.clone(),
        new_name: new.clone(),
    });

    backend_client.name_change(name_change_request).await?;

    monitor_config.change_avs_name(&old, &new)?;
    Ok(())
}

pub async fn start_monitor(mut config: IvyConfig) -> Result<(), anyhow::Error> {
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
        create_channel(backend_url, backend_ca).await.expect("Cannot create channel"),
    );

    info!("Starting monitor listener...");
    listen(backend_client, machine_id, identity_wallet, &monitor_config.configured_avses).await?;
    Ok(())
}

/// Scan function to set up configured AVS cache file. Derives `NodeType` from the name on the
/// metrics port and node name from the container name list.
pub async fn scan(force: bool, config: &IvyConfig) -> Result<(), anyhow::Error> {
    let backend_url = config.get_server_url()?;
    let backend_ca = config.get_server_ca();
    let backend_ca = if backend_ca.is_empty() { None } else { Some(backend_ca) };

    let backend = BackendClient::new(
        create_channel(backend_url, backend_ca)
            .await
            .map_err(|e| anyhow!("Failed to create backend channel: {}", e))?,
    );

    let mut monitor_config = MonitorConfig::load_from_default_path().unwrap_or_default();
    let configured_avs_names: HashSet<_> =
        monitor_config.configured_avses.iter().map(|a| a.container_name.clone()).collect();

    let potential_avses = grab_potential_avses().await;
    let (new_avses, leftover_potential_avses) =
        find_new_avses(&potential_avses, backend, &configured_avs_names).await?;

    if !force && new_avses.is_empty() {
        println!("No potential new AVSes found");
        return Ok(());
    }

    let selected_avses = select_avses(&new_avses, &leftover_potential_avses)?;
    if selected_avses.is_empty() {
        println!("No AVSes selected");
        return Ok(());
    }

    update_monitor_config(&mut monitor_config, selected_avses)?;
    println!("New setup stored with {} AVSes configured", monitor_config.configured_avses.len());

    Ok(())
}

async fn find_new_avses(
    potential_avses: &[PotentialAvs],
    mut backend: BackendClient<Channel>,
    configured_names: &HashSet<String>,
) -> Result<(Vec<ConfiguredAvs>, Vec<PotentialAvs>), anyhow::Error> {
    let node_type_queries = potential_avses
        .iter()
        .map(|avs| NodeTypeQuery {
            image_name: avs.image_name.clone(),
            image_digest: avs.image_hash.clone(),
            container_name: avs.container_name.clone(),
        })
        .collect::<Vec<_>>();

    let resp = backend
        .node_type_queries(Request::new(NodeTypeQueries { node_types: node_type_queries }))
        .await?
        .into_inner();

    info!("{:#?}", resp);

    // Map of container name to node type
    let container_node_types: HashMap<String, String> =
        resp.node_types.into_iter().map(|nt| (nt.container_name, nt.node_type)).collect();

    let mut new_potential_avses = Vec::new();
    let mut new_configured_avses = Vec::new();

    info!("{:#?}", potential_avses);

    for avs in potential_avses {
        let node_type = container_node_types.get(&avs.container_name).cloned();

        let node_type = if node_type.is_none() || Some("unknown") == node_type.as_deref() {
            new_potential_avses.push(avs.clone());
            continue;
        } else {
            node_type.ok_or(anyhow!("Unexpected error when fetching node type: {:?}", avs))?
        };

        let metric_port = get_metrics_port(&avs.ports).await?;

        let new_avs = ConfiguredAvs {
            assigned_name: String::new(),
            container_name: avs.container_name.clone(),
            avs_type: node_type,
            metric_port,
        };

        if configured_names.contains(&avs.container_name) {
            new_configured_avses.push(new_avs);
        } else {
            new_potential_avses.push(PotentialAvs {
                container_name: avs.container_name.clone(),
                image_name: avs.image_name.clone(),
                image_hash: avs.image_hash.clone(),
                ports: avs.ports.clone(),
            });
        }
    }

    Ok((new_configured_avses, new_potential_avses))
}

async fn get_metrics_port(ports: &[u16]) -> Result<Option<u16>, anyhow::Error> {
    for &port in ports {
        if let Ok(metrics) = fetch_telemetry_from(port).await {
            if !metrics.is_empty() {
                return Ok(Some(port));
            }
        }
    }
    Ok(None)
}

fn select_avses(
    avses: &[ConfiguredAvs],
    leftover_potential_avses: &[PotentialAvs],
) -> Result<Vec<ConfiguredAvs>, anyhow::Error> {
    let mut selected_avses =
        if avses.is_empty() { Vec::new() } else { select_detected_avses(avses)? };

    if !leftover_potential_avses.is_empty() && should_add_manual_avses()? {
        selected_avses.extend(select_manual_avses(leftover_potential_avses)?);
    }

    if selected_avses.is_empty() {
        return Err(anyhow!("No AVSes were selected"));
    }

    Ok(selected_avses)
}

fn select_detected_avses(avses: &[ConfiguredAvs]) -> Result<Vec<ConfiguredAvs>, anyhow::Error> {
    debug_assert!(!avses.is_empty(), "avses must not be empty");

    let items: Vec<String> = avses
        .iter()
        .map(|a| format!("{} under container {}", a.avs_type, a.container_name))
        .collect();

    let selected = MultiSelect::new()
        .with_prompt("Select detected AVSes (SPACE to select, ENTER to confirm)")
        .items(&items)
        .interact()
        .map_err(|e| anyhow!("Selection failed: {}", e))?;

    Ok(selected.into_iter().map(|idx| avses[idx].clone()).collect())
}

fn should_add_manual_avses() -> Result<bool, anyhow::Error> {
    dialoguer::Confirm::new()
        .with_prompt("Would you like to manually add undetected AVSes?")
        .default(false) // Makes pressing enter equivalent to 'n'
        .interact()
        .map_err(|e| anyhow!("Selection failed: {}", e))
}

fn select_manual_avses(
    potential_avses: &[PotentialAvs],
) -> Result<Vec<ConfiguredAvs>, anyhow::Error> {
    debug_assert!(!potential_avses.is_empty(), "potential_avses must not be empty");

    let items: Vec<String> = potential_avses
        .iter()
        .map(|a| format!("{} under container {}", a.image_name, a.container_name))
        .collect();

    let selected = MultiSelect::new()
        .with_prompt("Select AVSes to add manually (SPACE to select, ENTER to confirm)")
        .items(&items)
        .interact()
        .map_err(|e| anyhow!("Selection failed: {}", e))?;

    Ok(selected
        .into_iter()
        .map(|idx| ConfiguredAvs {
            assigned_name: String::new(),
            container_name: potential_avses[idx].container_name.to_string(),
            avs_type: "unknown".to_string(),
            metric_port: None,
        })
        .collect())
}

fn update_monitor_config(
    config: &mut MonitorConfig,
    mut new_avses: Vec<ConfiguredAvs>,
) -> Result<(), anyhow::Error> {
    let mut seen_names: HashSet<String> =
        config.configured_avses.iter().map(|a| a.assigned_name.clone()).collect();

    for avs in &mut new_avses {
        loop {
            let assigned_name: String = dialoguer::Input::new()
                .with_prompt(format!("Enter a unique name for AVS {}", avs.container_name))
                .interact_text()
                .map_err(|e| anyhow!("Failed to get input: {}", e))?;

            if seen_names.contains(&assigned_name) {
                println!("Error: Name '{}' is already in use", assigned_name);
                continue;
            }

            seen_names.insert(assigned_name.clone());
            avs.assigned_name = assigned_name;
            break;
        }
    }

    config.configured_avses.extend(new_avses);
    config.store().map_err(|e| anyhow!("Failed to store config: {}", e))?;

    Ok(())
}

async fn grab_potential_avses() -> Vec<PotentialAvs> {
    let docker = DockerClient::default();
    info!("Scanning for containers, use LOG_LEVEL=debug to see images");
    let images = docker.list_images().await;
    debug!("images: {:#?}", images);
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
                        image_name: image_name.clone(),
                        image_hash: image_hash.to_string(),
                        ports,
                    });
                } else if let Some(key) = images.keys().find(|key| key.contains(&image_name)) {
                    debug!("SHOULD BE: No version tag image: {}", image_name);
                    let image_hash = images.get(key).unwrap();
                    debug!("key (should be with version tag, and its what we'll use for potential avs): {}", key);
                    return Some(PotentialAvs {
                        container_name: names.first().unwrap_or(&image_name).to_string(),
                        image_name: key.clone(),
                        image_hash: image_hash.to_string(),
                        ports,
                    });
                }
            }
            None
        })
        .collect::<Vec<_>>();

    potential_avses
}
