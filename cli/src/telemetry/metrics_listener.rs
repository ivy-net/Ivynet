use std::{collections::HashMap, sync::Arc, time::Duration};

use ivynet_docker::dockerapi::DockerApi;
use ivynet_grpc::messages::{Metrics, NodeDataV2};
use ivynet_signer::sign_utils::IvySigningError;
use kameo::{message::Message, Actor};
use reqwest::Client;
use tracing::{debug, error, info, warn};

use crate::{
    ivy_machine::{IvyMachine, MachineIdentityError, SysInfo},
    telemetry::dispatch::TelemetryMsg,
};

use super::{
    dispatch::{TelemetryDispatchError, TelemetryDispatchHandle},
    parser::TelemetryParser,
    ConfiguredAvs, ErrorChannelTx,
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

#[derive(Clone, Debug)]
pub struct MetricsListenerHandle<D: DockerApi> {
    actor: kameo::actor::ActorRef<MetricsListener<D>>,
}

impl<D: DockerApi> MetricsListenerHandle<D> {
    pub fn new(
        docker: &D,
        machine: IvyMachine,
        avses: &[ConfiguredAvs],
        dispatch: &TelemetryDispatchHandle,
        error_tx: ErrorChannelTx,
    ) -> Self {
        let listener = MetricsListener::new(
            docker.clone(),
            machine,
            avses.to_vec(),
            dispatch.clone(),
            error_tx,
        );
        let actor = kameo::actor::spawn(listener);
        Self { actor }
    }

    pub async fn tell_add_node(&self, avs: ConfiguredAvs) -> Result<(), MetricsListenerError> {
        self.actor.tell(MetricsMsg::AddNode(avs)).await.map_err(Into::into)
    }

    pub async fn ask_add_node(&self, avs: ConfiguredAvs) -> Result<(), MetricsListenerError> {
        self.actor.ask(MetricsMsg::AddNode(avs)).await.map_err(Into::into)
    }

    pub async fn tell_remove_node(&self, avs: ConfiguredAvs) -> Result<(), MetricsListenerError> {
        self.actor.tell(MetricsMsg::RemoveNode(avs)).await.map_err(Into::into)
    }

    pub async fn ask_remove_node(&self, avs: ConfiguredAvs) -> Result<(), MetricsListenerError> {
        self.actor.ask(MetricsMsg::RemoveNode(avs)).await.map_err(Into::into)
    }

    pub async fn tell_remove_node_by_name(
        &self,
        container_name: String,
    ) -> Result<(), MetricsListenerError> {
        self.actor.tell(MetricsMsg::RemoveNodeByName(container_name)).await.map_err(Into::into)
    }

    pub async fn ask_remove_node_by_name(
        &self,
        container_name: String,
    ) -> Result<(), MetricsListenerError> {
        self.actor.ask(MetricsMsg::RemoveNodeByName(container_name)).await.map_err(Into::into)
    }

    pub async fn tell_broadcast(&self) -> Result<(), MetricsListenerError> {
        self.actor.tell(MetricsMsg::Broadcast).await.map_err(Into::into)
    }

    pub async fn ask_broadcast(&self) -> Result<(), MetricsListenerError> {
        self.actor.ask(MetricsMsg::Broadcast).await.map_err(Into::into)
    }
}

impl<D: DockerApi> Message<MetricsMsg> for MetricsListener<D> {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: MetricsMsg,
        _: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            MetricsMsg::AddNode(avs) => {
                let cache_entry = AvsStateCache::new(avs.avs_type.to_string());
                self.avs_cache.insert(avs.clone(), cache_entry);
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
            MetricsMsg::RemoveNode(configured_avs) => {
                let avs_num = self.avses.len();
                self.avs_cache.remove(&configured_avs);
                self.avses.retain(|x| x.container_name != configured_avs.container_name);
                if avs_num != self.avses.len() {
                    info!("Detected container stop: {}", configured_avs.container_name);
                } else {
                    // Return early if no nodes were dropped due to an earlier removal.
                    // This will frequently happen on a docker down action, as the event
                    // stream sends 'stop', 'kill', and 'die' events in quick succession.
                    // Functionality reproduced in RemoveNodeByName as well.
                    return;
                }
            }
            MetricsMsg::RemoveNodeByName(container_name) => {
                let avs_num = self.avses.len();
                self.avs_cache.retain(|x, _| x.container_name != container_name);
                self.avses.retain(|x| x.container_name != container_name);
                if avs_num != self.avses.len() {
                    info!("Detected container stop: {}", container_name);
                } else {
                    return;
                }
            }
            MetricsMsg::Broadcast => {}
        }
        match self.broadcast_metrics().await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to broadcast metrics: {}", e);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum MetricsMsg {
    AddNode(ConfiguredAvs),
    RemoveNode(ConfiguredAvs),
    RemoveNodeByName(String),
    Broadcast,
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

    #[error(transparent)]
    MachineIdentityError(#[from] MachineIdentityError),

    #[error("Failed to broadcast metrics: {0}")]
    BroadcastError(#[from] kameo::error::SendError<MetricsMsg>),

    #[error(transparent)]
    SendError(#[from] kameo::error::SendError<TelemetryMsg>),
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

#[derive(Debug, Clone)]
pub struct AvsStateCache {
    node_type: Option<String>,
    version_hash: Option<String>,
    is_running: bool,
}

impl AvsStateCache {
    pub fn new(node_type: String) -> Self {
        Self { node_type: Some(node_type), version_hash: None, is_running: false }
    }

    pub fn update(&mut self, version_hash: Option<String>, is_running: bool) {
        self.version_hash = version_hash;
        self.is_running = is_running;
    }
}

/// The MetricsListener is responsible for listening to metrics from the machine and sending them
/// to the telemetry dispatch. It is also responsible for listening to changes in the AVS list and
/// updating the AVS list accordingly. `avses` would probably be better represented by a set keyed
/// to the container_name name, which is unique per docker sysem.

#[derive(Actor, Debug)]
pub struct MetricsListener<D: DockerApi> {
    docker: D,
    machine: IvyMachine,
    avses: Vec<ConfiguredAvs>,
    avs_cache: HashMap<ConfiguredAvs, AvsStateCache>,
    dispatch: TelemetryDispatchHandle,
    _error_tx: ErrorChannelTx,
    http_client: reqwest::Client,
}

impl<D: DockerApi> MetricsListener<D> {
    fn new(
        docker: D,
        machine: IvyMachine,
        avses: Vec<ConfiguredAvs>,
        dispatch: TelemetryDispatchHandle,
        _error_tx: ErrorChannelTx,
    ) -> Self {
        let mut avs_cache = HashMap::new();
        for avs in &avses {
            avs_cache.insert(avs.clone(), AvsStateCache::new(avs.avs_type.to_string()));
        }
        Self {
            docker,
            machine,
            avses,
            avs_cache,
            dispatch,
            _error_tx,
            http_client: reqwest::Client::new(),
        }
    }

    async fn broadcast_metrics(&mut self) -> Result<(), MetricsListenerError> {
        report_metrics(
            &self.docker,
            &self.machine,
            self.avses.as_slice(),
            &self.dispatch,
            &mut self.avs_cache,
            &self.http_client,
        )
        .await
    }
}

pub async fn report_metrics(
    docker: &impl DockerApi,
    machine: &IvyMachine,
    avses: &[ConfiguredAvs],
    dispatch: &TelemetryDispatchHandle,
    avs_cache: &mut HashMap<ConfiguredAvs, AvsStateCache>,
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

            let signed_metrics = machine.sign_metrics(Some(avs.assigned_name.clone()), &metrics)?;
            dispatch.tell(TelemetryMsg::Metrics(signed_metrics)).await?;

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

        // Send node data
        let node_data = NodeDataV2 {
            name: avs.assigned_name.to_string(),
            node_type: cache_entry.node_type.clone(),
            manifest: Some(version_hash.clone()),
            metrics_alive: Some(!metrics.is_empty()),
            node_running: Some(is_running),
        };

        let mut new_cache_entry = cache_entry.clone();
        new_cache_entry.update(Some(version_hash), is_running);

        let signed_node_data = machine.sign_node_data_v2(&node_data)?;

        dispatch.tell(TelemetryMsg::SignedNodeData(signed_node_data)).await?;
        avs_cache.insert(avs.clone(), new_cache_entry);
    }
    // Last but not least - send system metrics
    let system_metrics = fetch_system_telemetry();
    let signed_metrics = machine.sign_metrics(None, &system_metrics)?;
    dispatch.tell(TelemetryMsg::Metrics(signed_metrics)).await?;

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
    let SysInfo {
        cpu_cores,
        cpu_usage,
        memory_usage,
        memory_free,
        disk_usage,
        disk_free,
        uptime,
        ..
    } = SysInfo::from_system();

    vec![
        Metrics { name: "cpu_usage".to_owned(), value: cpu_usage, attributes: Default::default() },
        Metrics {
            name: "ram_usage".to_owned(),
            value: memory_usage as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "free_ram".to_owned(),
            value: memory_free as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "disk_usage".to_owned(),
            value: disk_usage as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "free_disk".to_owned(),
            value: disk_free as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "cores".to_owned(),
            value: cpu_cores as f64,
            attributes: Default::default(),
        },
        Metrics { name: "uptime".to_owned(), value: uptime as f64, attributes: Default::default() },
    ]
}
