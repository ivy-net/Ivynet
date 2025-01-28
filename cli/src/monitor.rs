use anyhow::anyhow;
use dialoguer::{Input, MultiSelect, Select};
use ivynet_docker::dockerapi::DockerClient;
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
use tracing::{error, info};

use crate::{
    config::{IvyConfig, DEFAULT_CONFIG_PATH},
    init::set_backend_connection,
    node_source::NodeSource,
    telemetry::{listen, metrics_listener::fetch_telemetry_from, ConfiguredAvs},
};

const MONITOR_CONFIG_FILE: &str = "monitor-config.toml";

#[derive(Clone, Debug)]
pub struct PotentialAvs {
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

pub async fn start_monitor(config: IvyConfig) -> Result<(), anyhow::Error> {
    if config.identity_wallet().is_err() {
        return Err(anyhow!(
            "No identity wallet found in config. Please configure your machine with ivynet scan"
        ));
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
pub async fn scan(force: bool, mut config: IvyConfig) -> Result<(), anyhow::Error> {
    if config.identity_wallet().is_err() {
        set_backend_connection(&mut config).await?;
    }
    let backend_url = config.get_server_url()?;
    let backend_ca = config.get_server_ca();
    let backend_ca = if backend_ca.is_empty() { None } else { Some(backend_ca) };

    let mut backend = BackendClient::new(
        create_channel(backend_url, backend_ca)
            .await
            .map_err(|e| anyhow!("Failed to create backend channel: {}", e))?,
    );

    let mut monitor_config = MonitorConfig::load_from_default_path().unwrap_or_default();

    let docker_client = DockerClient::default();
    let potential_docker_nodes = docker_client.potential_nodes().await;

    info!("POTENTIAL: {:#?}", potential_docker_nodes);
    let (_existing_nodes, new_configured_nodes, leftover_potential_nodes) =
        find_new_avses(&mut backend, &monitor_config.configured_avses, &potential_docker_nodes)
            .await?;

    if !force && new_configured_nodes.is_empty() {
        println!("No potential new AVSes found");
    }

    let selected_avses = select_avses(&new_configured_nodes, &leftover_potential_nodes)?;
    if selected_avses.is_empty() {
        println!("No AVSes selected");
        return Ok(());
    }

    update_monitor_config(&mut monitor_config, selected_avses)?;
    info!("New setup stored with {} AVSes configured", monitor_config.configured_avses.len());

    Ok(())
}

/// Compares a list of configured nodes to a list of potential nodes. Updates the configured nodes
/// if a potential node is found with the same container name. If a potential node is found with a
/// different container name, and a valid node type, it is added to a list of new configured nodes.
/// Otherwise, the potential node is added to a list of leftover potential nodes.
///
/// Returns a tuple of (updated_existing_nodes, new_valid_nodes, leftover_potential_nodes)
async fn find_new_avses(
    backend: &mut BackendClient<Channel>,
    configured_avses: &[ConfiguredAvs],
    potential_avses: &[PotentialAvs],
) -> Result<(Vec<ConfiguredAvs>, Vec<ConfiguredAvs>, Vec<ConfiguredAvs>), anyhow::Error> {
    let mut configured_nodes = configured_avses.to_vec();
    let mut new_configured_nodes = Vec::new();
    let mut leftover_potential_nodes = Vec::new();

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

    // Map of container name to node type
    let container_node_types: HashMap<String, String> =
        resp.node_types.into_iter().map(|nt| (nt.container_name, nt.node_type)).collect();

    for avs in potential_avses {
        let node_type =
            container_node_types.get(&avs.container_name).cloned().unwrap_or("unknown".to_string());
        let metric_port = get_metrics_port(&avs.ports).await?;
        let new_avs = ConfiguredAvs {
            assigned_name: avs.image_name.clone(),
            container_name: avs.container_name.clone(),
            avs_type: node_type,
            metric_port,
        };

        // update the existing configured AVS if it exists, otherwise push to new vec
        if let Some(node) =
            configured_nodes.iter_mut().find(|a| a.container_name == new_avs.container_name)
        {
            node.avs_type = new_avs.avs_type;
            node.metric_port = new_avs.metric_port;
        } else if new_avs.avs_type != "unknown" {
            new_configured_nodes.push(new_avs);
        } else {
            leftover_potential_nodes.push(new_avs);
        }
    }

    Ok((configured_nodes, new_configured_nodes, leftover_potential_nodes))
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
    leftover_potential_avses: &[ConfiguredAvs],
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
    potential_avses: &[ConfiguredAvs],
) -> Result<Vec<ConfiguredAvs>, anyhow::Error> {
    debug_assert!(!potential_avses.is_empty(), "potential_avses must not be empty");

    let items: Vec<String> = potential_avses
        .iter()
        .map(|a| format!("{} under container {}", a.assigned_name, a.container_name))
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
