use std::{collections::HashMap, sync::Arc, time::Duration};

use ivynet_docker::dockerapi::DockerApi;
use ivynet_grpc::messages::{
    Metrics, NodeData, NodeDataV2, SignedMetrics, SignedNodeData, SignedNodeDataV2,
};
use ivynet_signer::{
    sign_utils::{sign_metrics, sign_node_data, sign_node_data_v2, IvySigningError},
    IvyWallet,
};
use reqwest::Client;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::system::get_detailed_system_information;

use super::{
    dispatch::{TelemetryDispatchError, TelemetryDispatchHandle},
    parser::TelemetryParser,
    ConfiguredAvs, ErrorChannelTx, TelemetryError,
};

const TELEMETRY_INTERVAL_IN_MINUTES: u64 = 1;

/**
 * ----------METRICS LISTENER----------
 * The MetricsListener is responsible for querying and transmitting metrics updates from
 * metrics-enabled docker containers to the telemetry dispatch (see `telemetry/dispatch.rs`).
 *
 * The metrics listener broadcasts metrics updates at a regular interval, as well as when it
 * receives a signal to add or remove a node from its list of monitored AVSes.
 *
 * MetricsListenerHandle can be safely cloned and shared across threads.
 *
 * -- Initialization --
 * The MetricsListener is initialized with via the `MetricsListenerHandle::new` function, which
 * spawns a new tokio task to run the MetricsListener. The MetricsListener is initialized with
 * the `machine_id` and `identity_wallet` for signing and authentication, a list of `avses`
 * which are the AVS configurations for the containers to monitor, a `dispatch` handle for
 * sending the metrics to the telemetry dispatch, and an `error_tx` for sending errors to the
 * main thread (or wherever errors are being handled).
 *
 */

#[derive(Debug, Clone)]
pub struct MetricsListenerHandle {
    tx: mpsc::Sender<MetricsListenerAction>,
}

impl MetricsListenerHandle {
    pub fn new(
        docker: &impl DockerApi,
        machine_id: Uuid,
        identity_wallet: &IvyWallet,
        avses: &[ConfiguredAvs],
        dispatch: &TelemetryDispatchHandle,
        error_tx: ErrorChannelTx,
    ) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let listener = MetricsListener::new(
            docker.clone(),
            machine_id,
            identity_wallet.clone(),
            avses.to_vec(),
            dispatch.clone(),
            rx,
            error_tx,
        );
        tokio::spawn(listener.run());
        Self { tx }
    }

    pub async fn add_node(&self, avs: &ConfiguredAvs) -> Result<(), MetricsListenerHandleError> {
        self.tx.send(MetricsListenerAction::AddNode(avs.clone())).await?;
        Ok(())
    }

    pub async fn remove_node(&self, avs: &ConfiguredAvs) -> Result<(), MetricsListenerHandleError> {
        self.tx.send(MetricsListenerAction::RemoveNode(avs.clone())).await?;
        Ok(())
    }

    pub async fn remove_node_by_name(
        &self,
        container_name: &str,
    ) -> Result<(), MetricsListenerHandleError> {
        self.tx.send(MetricsListenerAction::RemoveNodeByName(container_name.to_string())).await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MetricsListenerHandleError {
    #[error("Failed to send metrics listener action: {0}")]
    SendError(#[from] mpsc::error::SendError<MetricsListenerAction>),
}

#[derive(Debug, thiserror::Error)]
pub enum MetricsListenerError {
    #[error("Failed to fetch metrics from container {container_name} on port {port}: {source}")]
    FetchError { container_name: String, port: u16, source: MetricsFetchError },

    #[error("Failed to sign metrics: {0}")]
    SigningError(#[from] Arc<IvySigningError>),

    #[error("Failed to dispatch metrics: {0}")]
    DispatchError(#[from] TelemetryDispatchError),

    #[error("Cache missing for container: {0}")]
    CacheMissing(String),
}

#[derive(Debug, thiserror::Error)]
pub enum MetricsFetchError {
    #[error("Failed to connect to metrics endpoint: {0}")]
    ConnectionError(#[from] reqwest::Error),

    #[error("Timeout while fetching metrics after {timeout_secs} seconds")]
    TimeoutError {
        timeout_secs: u64,
        #[source]
        source: reqwest::Error,
    },

    #[error("Failed to parse metrics response: {0}")]
    ParseError(String),

    #[error("Invalid metric format in line {line_number}: {line}")]
    InvalidMetricFormat { line_number: usize, line: String },
}

/// The MetricsListener is responsible for listening to metrics from the machine and sending them
/// to the telemetry dispatch. It is also responsible for listening to changes in the AVS list and
/// updating the AVS list accordingly. `avses` would probably be better represented by a set keyed
/// to the container_name name, which is unique per docker sysem.
pub struct MetricsListener<D: DockerApi> {
    docker: D,
    machine_id: Uuid,
    identity_wallet: IvyWallet,
    avses: Vec<ConfiguredAvs>,
    avs_cache: HashMap<ConfiguredAvs, (Option<String>, Option<String>, bool)>,
    dispatch: TelemetryDispatchHandle,
    rx: mpsc::Receiver<MetricsListenerAction>,
    error_tx: ErrorChannelTx,
    http_client: reqwest::Client,
}

impl<D: DockerApi> MetricsListener<D> {
    fn new(
        docker: D,
        machine_id: Uuid,
        identity_wallet: IvyWallet,
        avses: Vec<ConfiguredAvs>,
        dispatch: TelemetryDispatchHandle,
        rx: mpsc::Receiver<MetricsListenerAction>,
        error_tx: ErrorChannelTx,
    ) -> Self {
        let mut avs_cache = HashMap::new();
        for avs in &avses {
            avs_cache.insert(avs.clone(), (Some(avs.avs_type.to_string()), None, false));
        }
        Self {
            docker,
            machine_id,
            identity_wallet,
            avses,
            avs_cache,
            dispatch,
            rx,
            error_tx,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn run(mut self) {
        let mut interval =
            tokio::time::interval(Duration::from_secs(60 * TELEMETRY_INTERVAL_IN_MINUTES));
        // broadcast metrics when we get an update event or once a minute, whichever comes first
        loop {
            let res = tokio::select! {
                _ = interval.tick() => {
                    self.broadcast_metrics().await
                }
                Some(action) = self.rx.recv() => {
                    self.handle_action(action).await
                }
            };
            if let Err(e) = res {
                let _ = self.error_tx.send(TelemetryError::MetricsListenerError(e.into()));
            }
        }
    }

    async fn broadcast_metrics(&mut self) -> Result<(), MetricsListenerError> {
        report_metrics(
            &self.docker,
            self.machine_id,
            &self.identity_wallet,
            self.avses.as_slice(),
            &self.dispatch,
            &mut self.avs_cache,
            &self.http_client,
        )
        .await
    }

    async fn handle_action(
        &mut self,
        action: MetricsListenerAction,
    ) -> Result<(), MetricsListenerError> {
        match action {
            MetricsListenerAction::AddNode(avs) => {
                self.avs_cache.insert(avs.clone(), (Some(avs.avs_type.to_string()), None, false));
                // if container with name already exists, replace avs_type and metric_port
                if let Some(existing) =
                    self.avses.iter_mut().find(|x| x.container_name == avs.container_name)
                {
                    existing.avs_type = avs.avs_type;
                    existing.metric_port = avs.metric_port;
                } else {
                    self.avses.push(avs.clone());
                    info!("Added metrics listener for container: {}", avs.container_name);
                }
            }
            MetricsListenerAction::RemoveNode(avs) => {
                let avs_num = self.avses.len();
                self.avs_cache.remove(&avs);
                self.avses.retain(|x| x.container_name != avs.container_name);
                if avs_num != self.avses.len() {
                    info!("Detected container stop: {}", avs.container_name);
                } else {
                    // Return early if no nodes were dropped due to an earlier removal.
                    // This will frequently happen on a docker down action, as the event
                    // stream sends 'stop', 'kill', and 'die' events in quick succession.
                    // Functionality reproduced below as well.
                    return Ok(());
                }
            }
            MetricsListenerAction::RemoveNodeByName(container_name) => {
                let avs_num = self.avses.len();
                self.avs_cache.retain(|x, _| x.container_name != container_name);
                self.avses.retain(|x| x.container_name != container_name);
                if avs_num != self.avses.len() {
                    info!("Detected container stop: {}", container_name);
                } else {
                    return Ok(());
                }
            }
        }
        self.broadcast_metrics().await
    }
}

#[derive(Clone, Debug)]
pub enum MetricsListenerAction {
    AddNode(ConfiguredAvs),
    RemoveNode(ConfiguredAvs),
    /// Remove a node by its container name
    RemoveNodeByName(String),
}

pub async fn report_metrics(
    docker: &impl DockerApi,
    machine_id: Uuid,
    identity_wallet: &IvyWallet,
    avses: &[ConfiguredAvs],
    dispatch: &TelemetryDispatchHandle,
    avs_cache: &mut HashMap<ConfiguredAvs, (Option<String>, Option<String>, bool)>,
    http_client: &reqwest::Client,
) -> Result<(), MetricsListenerError> {
    let images = docker.list_images().await;
    debug!("System Docker images: {:#?}", images);

    for avs in avses {
        let version_hash = match docker.find_container_by_name(&avs.container_name).await {
            None => {
                warn!(
                    "Container {} is configured but does not appear to be running. Skipping telemetry.",
                    avs.container_name
                );
                continue;
            }
            Some(container) => match container.image() {
                None => {
                    error!(
                        "Container {} is running but has no image. This should be unenterable.",
                        avs.container_name
                    );
                    continue;
                }
                Some(image_name) => images
                    .get(image_name)
                    .or_else(|| {
                        images
                            .keys()
                            .find(|key| key.contains(image_name))
                            .and_then(|key| images.get(key))
                    })
                    .cloned(),
            },
        };

        let Some(version_hash) = version_hash else {
            warn!(
                "Container {} is running but we could not locate a digest. Continuing.",
                avs.container_name
            );
            continue;
        };

        let metrics = if let Some(port) = avs.metric_port {
            let metrics: Vec<Metrics> =
                fetch_telemetry_from(http_client, &avs.container_name, port)
                    .await
                    .unwrap_or_default();

            let metrics_signature =
                sign_metrics(metrics.as_slice(), identity_wallet).map_err(Arc::new)?;
            let signed_metrics = SignedMetrics {
                machine_id: machine_id.into(),
                avs_name: Some(avs.assigned_name.clone()),
                signature: metrics_signature.to_vec(),
                metrics: metrics.to_vec(),
            };

            dispatch.send_metrics(signed_metrics).await?;

            metrics
        } else {
            Vec::new()
        };

        // Send node data

        info!(
            "Sending node data with version hash: {:#?} for avs: {}",
            version_hash, avs.assigned_name
        );

        let is_running = docker.is_running(&avs.container_name).await;

        let cache_entry = avs_cache
            .get(avs)
            .ok_or_else(|| MetricsListenerError::CacheMissing(avs.container_name.clone()))?;

        let (node_type, prev_version_hash, was_running) = &cache_entry;
        // Send node data
        let node_data = NodeDataV2 {
            name: avs.assigned_name.to_string(),
            node_type: node_type.clone(),
            manifest: if *prev_version_hash == Some(version_hash.clone()) {
                None
            } else {
                Some(version_hash.clone())
            },
            metrics_alive: Some(!metrics.is_empty()),
            node_running: if is_running != *was_running { Some(true) } else { None },
        };

        let node_data_signature =
            sign_node_data_v2(&node_data, identity_wallet).map_err(Arc::new)?;
        let signed_node_data = SignedNodeDataV2 {
            machine_id: machine_id.into(),
            signature: node_data_signature.to_vec(),
            node_data: Some(node_data),
        };

        dispatch.send_node_data(signed_node_data).await?;
        avs_cache.insert(avs.clone(), (None, Some(version_hash), is_running));
    }
    // Last but not least - send system metrics
    let system_metrics = fetch_system_telemetry();
    let metrics_signature =
        sign_metrics(system_metrics.as_slice(), identity_wallet).map_err(Arc::new)?;
    let signed_metrics = SignedMetrics {
        machine_id: machine_id.into(),
        avs_name: None,
        signature: metrics_signature.to_vec(),
        metrics: system_metrics.to_vec(),
    };
    dispatch.send_metrics(signed_metrics).await?;

    Ok(())
}

pub async fn fetch_telemetry_from(
    client: &Client,
    container_name: &str,
    port: u16,
) -> Result<Vec<Metrics>, MetricsListenerError> {
    const TIMEOUT_SECS: u64 = 10;

    let resp = client
        .get(format!("http://localhost:{}/metrics", port))
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                MetricsFetchError::TimeoutError { timeout_secs: TIMEOUT_SECS, source: e }
            } else {
                MetricsFetchError::ConnectionError(e)
            }
        })
        .map_err(|e| MetricsListenerError::FetchError {
            container_name: container_name.to_string(),
            port,
            source: e,
        })?;

    let body = resp.text().await.map_err(MetricsFetchError::ConnectionError).map_err(|e| {
        MetricsListenerError::FetchError {
            container_name: container_name.to_string(),
            port,
            source: e,
        }
    })?;

    let mut metrics = Vec::new();
    for (line_number, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        // Skip empty lines and Prometheus metadata/comment lines
        if trimmed.is_empty() || trimmed.starts_with("# ") {
            continue;
        }

        match TelemetryParser::new(line).parse() {
            Some(metric) => metrics.push(metric),
            None => {
                debug!(
                    "Failed to parse metric at line {}: '{}' for container {}",
                    line_number + 1,
                    line,
                    container_name
                );
            }
        }
    }

    if metrics.is_empty() {
        Err(MetricsListenerError::FetchError {
            container_name: container_name.to_string(),
            port,
            source: MetricsFetchError::ParseError("No valid metrics found in response".to_string()),
        })
    } else {
        Ok(metrics)
    }
}

fn fetch_system_telemetry() -> Vec<Metrics> {
    // Now we need to add basic metrics
    let (cores, cpu_usage, ram_usage, free_ram, disk_usage, free_disk, uptime) =
        get_detailed_system_information();

    vec![
        Metrics { name: "cpu_usage".to_owned(), value: cpu_usage, attributes: Default::default() },
        Metrics {
            name: "ram_usage".to_owned(),
            value: ram_usage as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "free_ram".to_owned(),
            value: free_ram as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "disk_usage".to_owned(),
            value: disk_usage as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "free_disk".to_owned(),
            value: free_disk as f64,
            attributes: Default::default(),
        },
        Metrics { name: "cores".to_owned(), value: cores as f64, attributes: Default::default() },
        Metrics { name: "uptime".to_owned(), value: uptime as f64, attributes: Default::default() },
    ]
}
