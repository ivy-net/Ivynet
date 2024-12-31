use crate::{
    error::IvyError,
    grpc::{backend::backend_client::BackendClient, tonic::transport::Channel},
    wallet::IvyWallet,
};
use bollard::secret::{EventMessage, EventMessageTypeEnum};
use dispatch::{TelemetryDispatchError, TelemetryDispatchHandle};
use ivynet_docker::dockerapi::{DockerClient, DockerStreamError};
use ivynet_node_type::NodeType;
use logs_listener::{ListenerData, LogsListenerHandle};
use metrics_listener::MetricsListenerHandle;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::broadcast,
    time::{sleep, Duration},
};
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod dispatch;
pub mod logs_listener;
pub mod metrics_listener;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfiguredAvs {
    pub assigned_name: String,
    pub container_name: String,
    pub avs_type: NodeType,
    pub metric_port: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct DockerStreamListener {
    pub docker: DockerClient,
    pub metrics_listener_handle: MetricsListenerHandle,
}

impl DockerStreamListener {
    pub fn new(metrics_listener: &MetricsListenerHandle) -> Self {
        Self { docker: DockerClient::default(), metrics_listener_handle: metrics_listener.clone() }
    }

    pub async fn run(self, known_nodes: Vec<ConfiguredAvs>) -> Result<(), DockerStreamError> {
        let docker = DockerClient::default();
        let mut docker_stream = docker.stream_events();
        while let Some(Ok(event)) = docker_stream.next().await {
            if event.typ == Some(EventMessageTypeEnum::CONTAINER) {
                if let Some(action) = event.action.as_deref() {
                    match action {
                        "start" => {
                            self.on_start(event, &known_nodes).await?;
                        }
                        "stop" | "kill" | "die" => {
                            self.on_stop(event).await?;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn on_start(
        &self,
        event: EventMessage,
        avses: &[ConfiguredAvs],
    ) -> Result<(), DockerStreamError> {
        let actor = event.actor.ok_or(DockerStreamError::MissingActor)?;
        let attributes = actor.attributes.ok_or(DockerStreamError::MissingAttributes)?;
        let container_name = attributes.get("name").ok_or(DockerStreamError::MissingAttributes)?;

        let container = match self.docker.find_container_by_name(container_name).await {
            Some(container) => container,
            None => {
                return Ok(());
            }
        };

        let metrics_port = match container.metrics_port().await {
            Some(port) => port,
            None => {
                // wait for metrics port to potentially come up
                sleep(Duration::from_secs(10)).await;
                match container.metrics_port().await {
                    Some(port) => port,
                    None => {
                        return Ok(());
                    }
                }
            }
        };

        let configured = match avses.iter().find(|avs| avs.container_name == *container_name) {
            Some(avs) => avs.clone(),
            None => ConfiguredAvs {
                assigned_name: "unknown".to_owned(),
                container_name: container_name.clone(),
                avs_type: NodeType::Unknown,
                metric_port: Some(metrics_port),
            },
        };

        info!("Found container: {}", container_name);

        if let Err(e) = self.metrics_listener_handle.add_node(configured).await {
            error!("Error adding node: {:?}", e);
        }
        Ok(())
    }

    pub async fn on_stop(&self, event: EventMessage) -> Result<(), DockerStreamError> {
        let actor = event.actor.ok_or(DockerStreamError::MissingActor)?;
        let attributes = actor.attributes.ok_or(DockerStreamError::MissingAttributes)?;
        let container_name = attributes.get("name").ok_or(DockerStreamError::MissingAttributes)?;

        self.metrics_listener_handle.remove_node_by_name(container_name).await;

        Ok(())
    }
}

pub async fn listen(
    backend_client: BackendClient<Channel>,
    machine_id: Uuid,
    identity_wallet: IvyWallet,
    avses: &[ConfiguredAvs],
) -> Result<(), IvyError> {
    let dispatch = TelemetryDispatchHandle::from_client(backend_client).await;
    let error_rx = dispatch.error_rx.resubscribe();
    let docker = DockerClient::default();

    // The logs listener spawns the future immediately and does not need to be awaited with
    // tokio::select!
    let mut logs_listener = LogsListenerHandle::new(dispatch.clone(), docker.clone());

    for node in avses {
        if let Some(container) = &docker.find_container_by_name(&node.container_name).await {
            let listener_data = ListenerData::new(
                container.clone(),
                node.clone(),
                machine_id,
                identity_wallet.clone(),
            );
            logs_listener.add_listener(listener_data).await;
        } else {
            warn!("Cannot find container {}.", node.container_name);
        }
    }

    // Metrics listener handle spans the metrics listener on init. Errors no longer cause the
    // program to stop, but instead throw an error msg up to console.
    let metrics_listener_handle =
        MetricsListenerHandle::new(machine_id, &identity_wallet, avses, &dispatch);

    // start docker stream listener to begin sending events to the metrics listener
    let docker_listener = DockerStreamListener::new(&metrics_listener_handle);

    tokio::spawn(docker_listener.run(avses.to_vec()));

    tokio::select! {
        _ = handle_telemetry_errors(error_rx) => Ok(())
    }
}

async fn handle_telemetry_errors(mut error_rx: broadcast::Receiver<TelemetryDispatchError>) {
    while let Ok(error) = error_rx.recv().await {
        error!("Received telemetry error: {}", error);
        sleep(Duration::from_secs(30)).await;
    }
}
