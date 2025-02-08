use std::{sync::Arc, time::Duration};

use ivynet_grpc::messages::Metrics;
use ivynet_signer::sign_utils::IvySigningError;
use kameo::{message::Message, Actor};
use reqwest::Client;
use tracing::{debug, error, info};

use crate::{
    ivy_machine::{IvyMachine, MachineIdentityError},
    telemetry::dispatch::TelemetryMsg,
};

use super::{
    dispatch::{TelemetryDispatchError, TelemetryDispatchHandle},
    parser::TelemetryParser,
    ConfiguredAvs, ErrorChannelTx,
};

/// Handle to the Metrics Listener actor. This handle can be cloned and shared across threads. It
/// exposes methods for adding and removing nodes from the list of monitored AVSes, as well as for
/// broadcasting metrics updates. Exposed function nomenclature is `tell_*` for fire-and-forget and
/// `ask_*` for request-response. TODO: request-response pattern is not currently meaninfully used.
#[derive(Clone, Debug)]
pub struct MetricsListenerHandle {
    actor: kameo::actor::ActorRef<MetricsListener>,
}

impl MetricsListenerHandle {
    pub fn new(
        machine: IvyMachine,
        avses: &[ConfiguredAvs],
        dispatch: &TelemetryDispatchHandle,
        error_tx: ErrorChannelTx,
    ) -> Self {
        let listener = MetricsListener::new(machine, avses.to_vec(), dispatch.clone(), error_tx);
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

impl Message<MetricsMsg> for MetricsListener {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: MetricsMsg,
        _: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        debug!("Received metrics message: {:?}", msg);
        match msg {
            MetricsMsg::AddNode(avs) => {
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

/// The MetricsListener is responsible for listening to metrics from the machine and sending them
/// to the telemetry dispatch. It is also responsible for listening to changes in the AVS list and
/// updating the AVS list accordingly. `avses` would probably be better represented by a set keyed
/// to the container_name name, which is unique per docker sysem.

#[derive(Actor, Debug)]
pub struct MetricsListener {
    machine: IvyMachine,
    avses: Vec<ConfiguredAvs>,
    dispatch: TelemetryDispatchHandle,
    _error_tx: ErrorChannelTx,
    http_client: reqwest::Client,
}

impl MetricsListener {
    fn new(
        machine: IvyMachine,
        avses: Vec<ConfiguredAvs>,
        dispatch: TelemetryDispatchHandle,
        _error_tx: ErrorChannelTx,
    ) -> Self {
        Self { machine, avses, dispatch, _error_tx, http_client: reqwest::Client::new() }
    }

    async fn broadcast_metrics(&mut self) -> Result<(), MetricsListenerError> {
        report_metrics(&self.machine, self.avses.as_slice(), &self.dispatch, &self.http_client)
            .await
    }
}

pub async fn report_metrics(
    machine: &IvyMachine,
    avses: &[ConfiguredAvs],
    dispatch: &TelemetryDispatchHandle,
    http_client: &reqwest::Client,
) -> Result<(), MetricsListenerError> {
    for avs in avses {
        if let Some(port) = avs.metric_port {
            let metrics: Vec<Metrics> =
                fetch_telemetry_from(http_client, &avs.container_name, port)
                    .await
                    .unwrap_or_default();

            let signed_metrics = machine.sign_metrics(Some(avs.assigned_name.clone()), &metrics)?;
            dispatch.tell(TelemetryMsg::Metrics(signed_metrics)).await?;
        }
    }
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
