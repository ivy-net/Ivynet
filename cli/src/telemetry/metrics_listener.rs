use std::{collections::HashMap, sync::Arc, time::Duration};

use ivynet_docker::dockerapi::DockerApi;
use ivynet_grpc::messages::{Metrics, NodeData, SignedMetrics, SignedNodeData};
use ivynet_signer::{
    sign_utils::{sign_metrics, sign_node_data, IvySigningError},
    IvyWallet,
};
use reqwest::Client;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{monitor::MonitorConfig, system::get_detailed_system_information};

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

    pub async fn replace_node(
        &self,
        old_avs: &ConfiguredAvs,
        new_avs: &ConfiguredAvs,
    ) -> Result<(), MetricsListenerHandleError> {
        self.tx.send(MetricsListenerAction::ReplaceNode(old_avs.clone(), new_avs.clone())).await?;
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
                    self.broadcast_metrics().await
                }
                Some(action) = self.rx.recv() => {
                    self.handle_action(action).await
                }
            };
            if let Err(e) = res {
                let _ = self.error_tx.send(e.into());
            }
        }
    }

    async fn broadcast_metrics(&mut self) -> Result<(), MetricsListenerError> {
        match report_metrics(
            &self.docker,
            self.machine_id,
            &self.identity_wallet,
            self.avses.as_slice(),
            &self.dispatch,
            &mut self.avs_cache,
        )
        .await?
        {
            Some((avs_to_replace, Some(replacement))) => {
                // TODO: This is a hack .This should be somehow marked in the loop process, but I
                // don't have an access to sender of the channel.
                // I'm open for suggestions how to handle it
                self.avs_cache.remove(&avs_to_replace);
                self.avses.retain(|x| x.container_name != avs_to_replace.container_name);
                self.avs_cache.insert(
                    replacement.clone(),
                    (Some(replacement.avs_type.to_string()), "".to_string(), false),
                );
                self.avses.push(replacement.clone());
                if let Ok(mut config) = MonitorConfig::load_from_default_path() {
                    _ = config.change_avs_container_name(
                        &avs_to_replace.container_name,
                        &replacement.container_name,
                    );
                }
                // We do nothning else here. In next round of the loop this will rename will be used
                // in the metrics queue
            }
            Some((avs_to_remove, None)) => {
                self.avs_cache.remove(&avs_to_remove);
                self.avses.retain(|x| x.container_name != avs_to_remove.container_name);
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_action(
        &mut self,
        action: MetricsListenerAction,
    ) -> Result<(), MetricsListenerError> {
        match action {
            MetricsListenerAction::AddNode(avs) => {
                if self.avses.iter().find(|a| a.container_name == *avs.container_name).is_none() {
                    self.avs_cache.insert(
                        avs.clone(),
                        (Some(avs.avs_type.to_string()), "".to_string(), false),
                    );
                    // if container with name already exists, replace avs_type and metric_port
                    if let Some(existing) =
                        self.avses.iter_mut().find(|x| x.container_name == avs.container_name)
                    {
                        existing.avs_type = avs.avs_type.clone();
                        existing.metric_port = avs.metric_port;
                    } else {
                        self.avses.push(avs.clone());
                        info!("Added metrics listener for container: {}", avs.container_name);
                    }
                    // We need to resave the monitor file
                    if let Ok(mut monitor_config) = MonitorConfig::load_from_default_path() {
                        if monitor_config
                            .configured_avses
                            .iter()
                            .find(|x| x.container_name == avs.container_name)
                            .is_none()
                        {
                            monitor_config.configured_avses.push(avs.clone());
                            _ = monitor_config.store();
                        }
                    } else {
                        error!("Cannot load monitor config for changes");
                    }
                }
            }
            MetricsListenerAction::ReplaceNode(old_avs, new_avs) => {
                self.avs_cache.remove(&old_avs);
                self.avses.retain(|x| x.container_name != old_avs.container_name);
                self.avs_cache.insert(
                    new_avs.clone(),
                    (Some(new_avs.avs_type.to_string()), "".to_string(), false),
                );
                self.avses.push(new_avs.clone());
                // Resaving the file
                if let Ok(mut monitor_config) = MonitorConfig::load_from_default_path() {
                    _ = monitor_config.change_avs_container_name(
                        &old_avs.container_name,
                        &new_avs.container_name,
                    );
                } else {
                    error!("Cannot load monitor config for changes");
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
    ReplaceNode(ConfiguredAvs, ConfiguredAvs),
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
) -> Result<Option<(ConfiguredAvs, Option<ConfiguredAvs>)>, MetricsListenerError> {
    let images = docker.list_images().await;
    debug!("System Docker images: {:#?}", images);

    for avs in avses {
        let version_hash = match docker.find_container_by_name(&avs.container_name).await {
            None => {
                // There is a chance that this container has been renamed.
                // We match it up using avs_type and if found, we break the loop
                // forcing avs list refresh with new container replacement
                if let Some(matched) = docker.find_container_by_image(&avs.image_name).await {
                    if let Some(names) = matched.names() {
                        if let Some(name) = names.into_iter().next() {
                            // We first need to check if we already have that name on the list not
                            // to duplicate
                            if avses.iter().find(|a| a.container_name == *name).is_none() {
                                let mut replacement = avs.clone();
                                replacement.container_name = name.clone();
                                info!(
                                    "Replacing container named {} with {}",
                                    avs.container_name, replacement.container_name
                                );
                                return Ok(Some((avs.clone(), Some(replacement))));
                            }
                        }
                    }
                }
                warn!(
                    "Container {} is configured but does not appear to be running. Skipping telemetry.",
                    avs.container_name
                );
                // If we reached here, we should remove this avs from the list
                return Ok(Some((avs.clone(), None)));
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

        info!(
            "Sending node data with version hash: {:#?} for avs: {}",
            version_hash, avs.assigned_name
        );

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

    Ok(None)
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
