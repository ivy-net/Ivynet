use anyhow::anyhow;
use dialoguer::{Input, MultiSelect, Select};
use ivynet_core::{
    config::{IvyConfig, DEFAULT_CONFIG_PATH},
    grpc::{
        self,
        backend::backend_client::BackendClient,
        messages::{Digests, NodeTypes, SignedNameChange},
        tonic::{transport::Channel, Request, Response},
    },
    io::{read_toml, write_toml, IoError},
    signature::sign_name_change,
    telemetry::{fetch_telemetry_from, listen, ConfiguredAvs},
};
use ivynet_docker::{dockerapi::DockerClient, RegistryType};
use ivynet_node_type::NodeType;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use tracing::info;

use crate::init::set_backend_connection;

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
        grpc::client::create_channel(backend_url, backend_ca).await.expect("Cannot create channel"),
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
        grpc::client::create_channel(backend_url, backend_ca).await.expect("Cannot create channel"),
    );

    info!("Starting monitor listener...");
    listen(backend_client, machine_id, identity_wallet, &monitor_config.configured_avses).await?;
    Ok(())
}

/// Scan function to set up configured AVS cache file. Derives `NodeType` from the name on the
/// metrics port and node name from the container name list.
pub async fn scan(config: &IvyConfig) -> Result<(), anyhow::Error> {
    let backend_url = config.get_server_url()?;
    let backend_ca = config.get_server_ca();
    let backend_ca = if backend_ca.is_empty() { None } else { Some(backend_ca) };

    let backend = BackendClient::new(
        grpc::client::create_channel(backend_url, backend_ca)
            .await
            .map_err(|e| anyhow!("Failed to create backend channel: {}", e))?,
    );

    let mut monitor_config = MonitorConfig::load_from_default_path().unwrap_or_default();
    let configured_avs_names: HashSet<_> =
        monitor_config.configured_avses.iter().map(|a| a.container_name.clone()).collect();

    let potential_avses = grab_potential_avses().await;
    let new_avses = find_new_avses(&potential_avses, backend, &configured_avs_names).await?;

    if new_avses.is_empty() {
        println!("No potential new AVSes found");
        return Ok(());
    }

    let selected_avses = select_avses(&new_avses)?;
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
) -> Result<Vec<ConfiguredAvs>, anyhow::Error> {
    let digests: Vec<_> = potential_avses
        .iter()
        .filter(|a| !configured_names.contains(&a.container_name))
        .map(|a| a.image_hash.clone())
        .collect();

    if digests.is_empty() {
        return Ok(Vec::new());
    }

    let node_types: Option<NodeTypes> = backend
        .node_types(Request::new(Digests { digests: digests.clone() }))
        .await
        .map(Response::into_inner)
        .ok();

    let avs_types: Option<HashMap<String, NodeType>> = if let Some(node_types) = node_types {
        Some(
            node_types
                .node_types
                .into_iter()
                .map(|nt| (nt.digest, NodeType::from(nt.node_type.as_str())))
                .collect::<HashMap<_, _>>(),
        )
    } else {
        None
    };

    let mut new_avses = Vec::new();
    for avs in potential_avses {
        if configured_names.contains(&avs.container_name) {
            continue;
        }

        if let Some(avs_type) =
            get_type(&avs_types, &avs.image_hash, &avs.image_name, &avs.container_name)
        {
            // Try to get metrics port but don't fail if unavailable
            let metric_port = match get_metrics_port(&avs.ports).await {
                Ok(port) => port,
                Err(e) => {
                    info!("Metrics unavailable for {}: {}", avs.container_name, e);
                    None
                }
            };

            new_avses.push(ConfiguredAvs {
                assigned_name: String::new(),
                container_name: avs.container_name.clone(),
                avs_type,
                metric_port,
            });
        }
    }

    Ok(new_avses)
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

fn select_avses(avses: &[ConfiguredAvs]) -> Result<Vec<ConfiguredAvs>, anyhow::Error> {
    let items: Vec<String> = avses
        .iter()
        .map(|a| format!("{} under container {}", a.avs_type, a.container_name))
        .collect();

    let selected = MultiSelect::new()
        .with_prompt("Select AVSes to add (SPACE to select, ENTER to confirm)")
        .items(&items)
        .interact()
        .map_err(|e| anyhow!("Selection failed: {}", e))?;

    Ok(selected.into_iter().map(|idx| avses[idx].clone()).collect())
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

fn get_type(
    hashes: &Option<HashMap<String, NodeType>>,
    hash: &str,
    image_name: &str,
    container_name: &str,
) -> Option<NodeType> {
    let node_type = hashes
        .clone()
        .and_then(|h| h.get(hash).copied())
        .or_else(|| NodeType::from_image(&extract_image_name(image_name)))
        .or_else(|| NodeType::from_default_container_name(container_name.trim_start_matches('/')));
    if node_type.is_none() {
        println!("No avs found for {}", image_name);
    }
    node_type
}

async fn grab_potential_avses() -> Vec<PotentialAvs> {
    let docker = DockerClient::default();
    println!("Scanning containers...");
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
                        image_name: image_name.clone(),
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

fn extract_image_name(image_name: &str) -> String {
    RegistryType::get_registry_hosts()
        .into_iter()
        .find_map(|registry| {
            image_name.contains(registry).then(|| {
                image_name
                    .split(&registry)
                    .last()
                    .unwrap_or(image_name)
                    .trim_start_matches('/')
                    .to_string()
            })
        })
        .unwrap_or_else(|| image_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_image_name() {
        let test_cases = vec![
            // Standard registry cases
            ("docker.io/ubuntu:latest", "ubuntu:latest"),
            ("gcr.io/project/image:v1", "project/image:v1"),
            ("ghcr.io/owner/repo:tag", "owner/repo:tag"),
            ("public.ecr.aws/image:1.0", "image:1.0"),
            // Edge cases
            ("ubuntu:latest", "ubuntu:latest"), // No registry
            ("", ""),                           // Empty string
            ("repository.chainbase.com/", ""),  // Just registry
            // Multiple registry-like strings
            ("gcr.io/docker.io/image", "image"), // Should match first registry
            // With and without tags
            ("docker.io/image", "image"),
            ("docker.io/org/image:latest", "org/image:latest"),
            // Special characters
            ("docker.io/org/image@sha256:123", "org/image@sha256:123"),
            ("docker.io/org/image_name", "org/image_name"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                extract_image_name(input),
                expected.to_string(),
                "Failed on input: {}",
                input
            );
        }
    }
}
