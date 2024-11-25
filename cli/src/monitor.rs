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

const IMAGE_NAME_EIGENDA: &str = "ghcr.io/layr-labs/eigenda/opr-node";
const METRIC_LABEL_PERFORMANCE: &str = "eigen_performance_score";
const METRIC_ATTR_LABEL_AVS_NAME: &str = "avs_name";
const MONITOR_CONFIG_FILE: &str = "monitor-config.toml";

#[derive(Clone, Debug)]
struct PotentialAvs {
    pub name: String,
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
        //Previous impl built a bad path - let this error properly
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

    let monitor_config = MonitorConfig::load_from_default_path().unwrap_or_default();
    if monitor_config.configured_avses.is_empty() {
        return Err(anyhow!("No AVSes configured to monitor"));
    }

    info!("Starting monitor listener...");
    listen(backend_client, machine_id, identity_wallet, &monitor_config.configured_avses).await?;
    Ok(())
}

pub async fn scan() -> Result<(), anyhow::Error> {
    let docker = DockerClient::default();
    println!("Scanning for existing containers...");
    let potential_avses = docker
        .list_containers()
        .await
        .into_iter()
        .filter_map(|c| {
            if let (Some(names), Some(image_name), Some(ports)) = (c.names, c.image, c.ports) {
                if let Some(avs_type) = potential_avs_name(&image_name) {
                    let ports = ports.into_iter().filter_map(|p| p.public_port).collect::<Vec<_>>();
                    if !ports.is_empty() {
                        return Some(PotentialAvs {
                            name: names.first().unwrap_or(&image_name).to_string(),
                            avs_type,
                            ports,
                        });
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>();

    let mut monitor_config = MonitorConfig::load_from_default_path().unwrap_or_default();
    let mut avses = Vec::new();

    let configured_avs_names =
        monitor_config.configured_avses.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
    for avs in &potential_avses {
        if !configured_avs_names.contains(&avs.name) {
            for port in &avs.ports {
                if let Ok(metrics) = fetch_telemetry_from(*port).await {
                    // Checking performance score metrics to read a potential avs type
                    avses.push(ConfiguredAvs {
                        name: avs.name.clone(),
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

    if avses.is_empty() {
        println!("No potential new AVSes found");
    } else {
        for idx in MultiSelect::new()
            .with_prompt("Choose what AVSes to add and accept the list with ENTER")
            .items(
                &avses
                    .iter()
                    .map(|a| format!("{} under container {}", a.avs_type, a.name))
                    .collect::<Vec<_>>(),
            )
            .interact()
            .expect("No items selected")
        {
            monitor_config.configured_avses.push(avses[idx].clone());
        }

        monitor_config.store()?;
        println!(
            "New setup stored with {} of avses configured",
            monitor_config.configured_avses.len()
        );
    }
    Ok(())
}

// TODO: Make NodeType api uniform here
fn potential_avs_name(name: &str) -> Option<NodeType> {
    if let NodeType::EigenDA = NodeType::from_docker_image_name(name) {
        return Some(NodeType::EigenDA);
    }
    None
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
