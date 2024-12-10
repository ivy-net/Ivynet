use anyhow::anyhow;
use dialoguer::MultiSelect;
use std::path::PathBuf;

use ivynet_core::{
    config::DEFAULT_CONFIG_PATH,
    docker::dockerapi::DockerClient,
    grpc::{self, backend::backend_client::BackendClient, messages::Metrics},
    io::{read_toml, write_toml, IoError},
    node_type::NodeType,
    telemetry::{fetch_telemetry_from, listen, ConfiguredAvs},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::init::set_backend_connection;

const METRIC_LABEL_PERFORMANCE: &str = "eigen_performance_score";
const METRIC_ATTR_LABEL_AVS_NAME: &str = "avs_name";
const MONITOR_CONFIG_FILE: &str = "monitor-config.toml";

#[derive(Clone, Debug)]
struct PotentialAvs {
    pub container_name: String,
    pub avs_type: NodeType,
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
        grpc::client::create_channel(grpc::client::Source::Uri(backend_url), backend_ca)
            .await
            .expect("Cannot create channel"),
    );

    info!("Starting monitor listener...");
    listen(backend_client, machine_id, identity_wallet, &monitor_config.configured_avses).await?;
    Ok(())
}

/// Scan function to set up configured AVS cache file. Derives `NodeType` from the name on the
/// metrics port and node name from the container name list.
pub async fn scan() -> Result<(), anyhow::Error> {
    let docker = DockerClient::default();
    println!("Scanning for existing containers...");
    let potential_avses = docker
        .list_containers()
        .await
        .into_iter()
        .filter_map(|c| {
            if let (Some(names), Some(image_name), Some(ports)) = (c.names, c.image, c.ports) {
                let avs_type = NodeType::from_image(&image_name).unwrap_or(NodeType::Unknown);
                let mut ports = ports.into_iter().filter_map(|p| p.public_port).collect::<Vec<_>>();

                if !ports.is_empty() {
                    ports.sort();
                    ports.dedup();
                    return Some(PotentialAvs {
                        container_name: names.first().unwrap_or(&image_name).to_string(),
                        avs_type,
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
    for avs in &potential_avses {
        if !configured_avs_names.contains(&avs.container_name) {
            for port in &avs.ports {
                if let Ok(metrics) = fetch_telemetry_from(*port).await {
                    if !metrics.is_empty() {
                        // Checking performance score metrics to read a potential avs type

                        avses.push(ConfiguredAvs {
                            assigned_name: "".to_string(),
                            container_name: avs.container_name.clone(),
                            avs_type: match guess_avs_type(metrics) {
                                NodeType::Unknown => avs.avs_type,
                                avs_type => avs_type,
                            },
                            metric_port: *port,
                        });
                    }
                }
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

        let mut seen_names = std::collections::HashSet::new();
        for avs in &mut monitor_config.configured_avses {
            let mut assigned_name;
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

                if configured_avs_names.contains(&assigned_name) {
                    println!(
                        "Error: Name '{}' is already configured. Please choose a unique name.",
                        assigned_name
                    );
                    continue;
                }

                seen_names.insert(assigned_name.clone());
                break;
            }
            avs.assigned_name = assigned_name;
        }

        monitor_config.store()?;
        println!(
            "New setup stored with {} of avses configured",
            monitor_config.configured_avses.len()
        );
    }
    Ok(())
}

fn guess_avs_type(metrics: Vec<Metrics>) -> NodeType {
    if let Some(name) =
        metrics
            .into_iter()
            .filter_map(|m| {
                if m.name == METRIC_LABEL_PERFORMANCE {
                    m.attributes
                        .into_iter()
                        .filter_map(|at| {
                            if at.name == METRIC_ATTR_LABEL_AVS_NAME {
                                Some(at)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .first()
                        .map(|n| n.value.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .first()
    {
        return NodeType::from_metrics_name(name);
    }

    NodeType::Unknown
}
