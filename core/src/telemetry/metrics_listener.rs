use std::{collections::HashMap, sync::Arc, time::Duration};

use ivynet_docker::dockerapi::DockerApi;
use reqwest::Client;
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    grpc::messages::{Metrics, NodeData, SignedMetrics, SignedNodeData},
    signature::{sign_metrics, sign_node_data, IvySigningError},
    system::get_detailed_system_information,
    wallet::IvyWallet,
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

#[derive(Clone, Debug, thiserror::Error)]
pub enum MetricsListenerError {
    #[error("Failed to fetch metrics from container: {0}")]
    MetricsFetchError(#[from] Arc<reqwest::Error>),
    #[error("Failed to sign metrics: {0}")]
    SigningError(#[from] Arc<IvySigningError>),
    #[error("Failed to send metrics: {0}")]
    DispatchError(#[from] TelemetryDispatchError),
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
    avs_cache: HashMap<ConfiguredAvs, (Option<String>, String, bool)>,
    dispatch: TelemetryDispatchHandle,
    rx: mpsc::Receiver<MetricsListenerAction>,
    error_tx: ErrorChannelTx,
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
            avs_cache.insert(avs.clone(), (Some(avs.avs_type.to_string()), "".to_string(), false));
        }
        Self { docker, machine_id, identity_wallet, avses, avs_cache, dispatch, rx, error_tx }
    }

    pub async fn run(mut self) {
        let mut interval =
            tokio::time::interval(Duration::from_secs(60 * TELEMETRY_INTERVAL_IN_MINUTES));
        // broadcast metrics when we get an update event or once a minute, whichever comes first
        loop {
            let res = tokio::select! {
                _ = interval.tick() => {
                    info!("Interval tick");
                    self.broadcast_metrics().await
                }
                Some(action) = self.rx.recv() => {
                    info!("Action received: {:#?}", action);
                    self.handle_action(action).await
                }
            };
            if let Err(e) = res {
                let _ = self.error_tx.send(e.into());
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
        )
        .await
    }

    async fn handle_action(
        &mut self,
        action: MetricsListenerAction,
    ) -> Result<(), MetricsListenerError> {
        match action {
            MetricsListenerAction::AddNode(avs) => {
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
                self.avses.retain(|x| x.container_name != avs.container_name);
            }
            MetricsListenerAction::RemoveNodeByName(container_name) => {
                let avs_num = self.avses.len();
                self.avses.retain(|x| x.container_name != container_name);
                if avs_num != self.avses.len() {
                    info!("Detected container stop: {}", container_name);
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
    avs_cache: &mut HashMap<ConfiguredAvs, (Option<String>, String, bool)>,
) -> Result<(), MetricsListenerError> {
    let images = docker.list_images().await;

    debug!("Got images {images:#?}");
    for avs in avses {
        let mut version_hash = "".to_string();
        if let Some(inspect_data) = docker.find_container_by_name(&avs.container_name).await {
            if let Some(image_name) = inspect_data.image() {
                if let Some(hash) = images.get(image_name) {
                    version_hash = hash.clone();
                }
            }
        }

        let metrics = if let Some(port) = avs.metric_port {
            let metrics: Vec<Metrics> = fetch_telemetry_from(port).await.unwrap_or_default();

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

        info!("Sending node data with version hash: {:#?}", version_hash);

        let is_running = docker.is_running(&avs.container_name).await;

        let (node_type, prev_version_hash, was_running) = &avs_cache[avs];
        // Send node data
        let node_data = NodeData {
            name: avs.assigned_name.to_string(),
            node_type: node_type.clone(),
            manifest: if *prev_version_hash == version_hash {
                None
            } else {
                Some(version_hash.clone())
            },
            metrics_alive: Some(!metrics.is_empty()),
            node_running: if is_running != *was_running { Some(true) } else { None },
        };

        let node_data_signature = sign_node_data(&node_data, identity_wallet).map_err(Arc::new)?;
        let signed_node_data = SignedNodeData {
            machine_id: machine_id.into(),
            signature: node_data_signature.to_vec(),
            node_data: Some(node_data),
        };

        dispatch.send_node_data(signed_node_data).await?;
        avs_cache.insert(avs.clone(), (None, version_hash, is_running));
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

pub async fn fetch_telemetry_from(port: u16) -> Result<Vec<Metrics>, MetricsListenerError> {
    let client = Client::new();
    let resp = client
        .get(format!("http://localhost:{}/metrics", port))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(Arc::new)?;
    let body = resp.text().await.map_err(Arc::new)?;
    let metrics =
        body.split('\n').filter_map(|line| TelemetryParser::new(line).parse()).collect::<Vec<_>>();
    Ok(metrics)
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
