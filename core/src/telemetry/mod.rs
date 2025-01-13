use crate::{
    error::IvyError,
    grpc::{backend::backend_client::BackendClient, tonic::transport::Channel},
    wallet::IvyWallet,
};
use dispatch::{TelemetryDispatchError, TelemetryDispatchHandle};
use docker_event_stream_listener::DockerStreamListener;
use ivynet_docker::dockerapi::{DockerApi, DockerClient};
use ivynet_node_type::NodeType;
use logs_listener::LogsListenerManager;
use metrics_listener::MetricsListenerHandle;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod dispatch;
pub mod docker_event_stream_listener;
pub mod logs_listener;
pub mod metrics_listener;
pub mod parser;

pub type ErrorChannelTx = broadcast::Sender<TelemetryError>;
pub type ErrorChannelRx = broadcast::Receiver<TelemetryError>;

#[derive(Clone, Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("Telemetry dispatch error: {0}")]
    DispatchError(TelemetryDispatchError),

    #[error("Docker stream error: {0}")]
    DockerStreamError(ivynet_docker::dockerapi::DockerStreamError),

    #[error("Telemetry dispatch error: {0}")]
    TelemetryDispatchError(#[from] TelemetryDispatchError),

    #[error("Metrics listener error: {0}")]
    MetricsListenerError(#[from] metrics_listener::MetricsListenerError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfiguredAvs {
    pub assigned_name: String,
    pub container_name: String,
    pub avs_type: NodeType,
    pub metric_port: Option<u16>,
}

/**
 * -------------NODE LISTENER COMPOSITION-------------
 * The listen function is the entry point for the telemetry module. It is responsible for
 * setting up the various listeners and dispatchers that will handle telemetry data.
 *
 * The listen function initializes and composes the following actors:
 *
 * 1. Dispatcher: The dispatcher is responsible for receiving telemetry data from the various
 *    other listeners via a tokio mpsc channel and sending it to the backend. It is the central
 *    hub for telemetry data transmission. Interface is accessible via the
 *    TelemetryDispatchHandle.
 *
 * 2. Logs Listener: The logs listener is responsible for listening to logs from containers and
 *    sending them to the dispatcher. It is composed of a LogsListenerManager and a set of
 *    LogsListeners. The LogsListenerManager is responsible for managing the set of listeners
 *    and spawning them as futures. The LogsListeners are responsible for listening to logs from
 *    a single container and sending them to the dispatcher. If a given LogsListener receives a
 *    signal that the docker log stream is closed, it shuts down and is removed from the managed
 *    list. The LogsListenerManager serves as the handle for the interior set of all logs
 *    listeners.
 *
 * 3. Metrics Listener: The metrics listener is responsible for listening to metrics from
 *    containers and sending them to the dispatcher. It receives an initial set of configured
 *    nodes and sends metrics for all containers in its set to the dispatcher at fixed
 *    intervals. Additionally, its list of nodes may be managed via the MetricsListenerHandle
 *    interface, and it will transmit metrics for all nodes in its set after each update in
 *    addition to the fixed interval.
 *
 * 4. Docker Stream Listener: The docker stream listener is responsible for listening to docker
 *    stream events and sending them to the other listeners for processing. It has no associated
 *    handle and is spawned as a future in the listen function.
 *
 */

pub async fn listen(
    backend_client: BackendClient<Channel>,
    machine_id: Uuid,
    identity_wallet: IvyWallet,
    avses: &[ConfiguredAvs],
) -> Result<(), IvyError> {
    let docker = DockerClient::default();

    let (error_tx, error_rx) = tokio::sync::broadcast::channel(64);

    // Telemtry dispatcher recieves telemetry messages from other listeners and sends them to the
    // backend
    let dispatch = TelemetryDispatchHandle::new(backend_client.clone(), &error_tx).await;

    // Logs Listener handles logs from containers and sends them to the dispatcher
    let mut logs_listener_handle =
        LogsListenerManager::new(&docker, &identity_wallet, machine_id, &dispatch);

    for node in avses {
        info!("Searching for node: {}", node.container_name);
        if let Some(container) = &docker.find_container_by_name(&node.container_name).await {
            if let Err(e) = logs_listener_handle.add_listener(container, node).await {
                error!("Failed to add logs listener for container: {}", e);
            };
        } else {
            warn!("Cannot find container for configured node: {}.", node.container_name);
        }
    }

    // Metrics Listener handles metrics from containers and sends them to the dispatcher
    let metrics_listener_handle = MetricsListenerHandle::new(
        &docker,
        machine_id,
        &identity_wallet,
        avses,
        &dispatch,
        error_tx,
    );

    // Stream listener listens for docker events and sends them to the other listeners for
    // processing
    let docker_listener =
        DockerStreamListener::new(metrics_listener_handle, logs_listener_handle, backend_client);
    tokio::spawn(docker_listener.run(avses.to_vec()));

    // This should never return unless the error channel is closed
    handle_telemetry_errors(error_rx).await?;

    Ok(())
}

async fn handle_telemetry_errors(mut error_rx: ErrorChannelRx) -> Result<(), IvyError> {
    while let Ok(error) = error_rx.recv().await {
        error!("Received telemetry error: {}", error);
    }
    Ok(())
}
