use std::time::Duration;

use bollard::secret::{EventMessage, EventMessageTypeEnum};
use ivynet_docker::dockerapi::{DockerApi, DockerClient, DockerStreamError};
use ivynet_grpc::{
    backend::backend_client::BackendClient,
    messages::{NodeTypeQueries, NodeTypeQuery},
    tonic::{transport::Channel, Request, Response},
};
use ivynet_signer::IvyWallet;
use tokio::time::sleep;
use tokio_stream::StreamExt;
use tracing::{debug, error};
use uuid::Uuid;

use super::{
    dispatch::{TelemetryDispatchError, TelemetryDispatchHandle},
    logs_listener::LogsListenerManager,
    metrics_listener::MetricsListenerHandle,
    ConfiguredAvs,
};

#[derive(Debug)]
pub struct DockerStreamListener<D: DockerApi> {
    pub docker: D,
    pub metrics_listener_handle: MetricsListenerHandle,
    pub logs_listener_handle: LogsListenerManager,
    pub dispatch: TelemetryDispatchHandle,
    pub machine_id: Uuid,
    pub identity_wallet: IvyWallet,
    pub backend: BackendClient<Channel>,
}

impl DockerStreamListener<DockerClient> {
    pub fn new(
        metrics_listener: MetricsListenerHandle,
        logs_listener: LogsListenerManager,
        dispatch: TelemetryDispatchHandle,
        identity_wallet: IvyWallet,
        machine_id: Uuid,
        backend: BackendClient<Channel>,
    ) -> Self {
        Self {
            docker: DockerClient::default(),
            metrics_listener_handle: metrics_listener,
            logs_listener_handle: logs_listener,
            dispatch,
            machine_id,
            identity_wallet,
            backend,
        }
    }

    pub async fn run(
        mut self,
        known_nodes: Vec<ConfiguredAvs>,
    ) -> Result<(), DockerStreamListenerError> {
        let mut docker_stream = self.docker.stream_events().await;
        while let Some(Ok(event)) = docker_stream.next().await {
            debug!("Dockerstream Event | {:?}", event);
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
        &mut self,
        event: EventMessage,
        avses: &[ConfiguredAvs],
    ) -> Result<(), DockerStreamListenerError> {
        let actor = event.actor.ok_or(DockerStreamError::MissingActor)?;
        let attributes = actor.attributes.ok_or(DockerStreamError::MissingAttributes)?;
        let inc_container_name =
            attributes.get("name").ok_or(DockerStreamError::MissingAttributes)?;

        let inc_container = match self.docker.find_container_by_name(inc_container_name).await {
            Some(container) => container,
            None => {
                return Ok(());
            }
        };

        let inc_image_name = inc_container.image().unwrap_or_default().to_string();
        let inc_container_digest = inc_container.image_id().unwrap_or_default().to_string();

        let metrics_port = match inc_container.metrics_port(&self.docker).await {
            Some(port) => Some(port),
            None => {
                // wait for metrics port to potentially come up
                sleep(Duration::from_secs(10)).await;
                inc_container.metrics_port(&self.docker).await
            }
        };

        let mut configured = None;
        for avs in avses {
            // First try to find by container name
            if avs.container_name == *inc_container_name {
                configured = Some(avs.clone());
                break;
            }
            // If not found by name, check if any existing AVS is monitoring
            // the same container (by image hash)
            if let Some(existing_container) =
                self.docker.find_container_by_name(&avs.container_name).await
            {
                if let Some(existing_digest) = existing_container.image_id() {
                    if existing_digest == inc_container_digest {
                        configured = Some(avs.clone());
                        break;
                    }
                }
            }
        }

        let configured = match configured {
            Some(avs) => Some(avs),
            None => {
                let node_type_query = NodeTypeQuery {
                    container_name: inc_container_name.clone(),
                    image_name: inc_image_name.clone(),
                    image_digest: inc_container_digest.clone(),
                };
                let query = Request::new(NodeTypeQueries { node_types: vec![node_type_query] });

                let response =
                    self.backend.node_type_queries(query).await.map(Response::into_inner).ok();

                // Only create configuration if we get a valid node type
                match response {
                    Some(node_type) => {
                        if let Some(node_type) = node_type.node_types.first() {
                            if node_type.node_type != "unknown" {
                                Some(ConfiguredAvs {
                                    assigned_name: inc_container_name.to_string(),
                                    container_name: inc_container_name.clone(),
                                    avs_type: node_type.node_type.clone(),
                                    metric_port: metrics_port,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
        };

        if let Some(configured) = configured {
            debug!("Found container: {}", inc_container_name);

            if let Err(e) = self.metrics_listener_handle.add_node(&configured).await {
                error!("Error adding node: {:?}", e);
            }
            if let Err(e) =
                self.logs_listener_handle.add_listener(&inc_container, &configured).await
            {
                error!("Error adding listener: {:?}", e);
            }
        }

        Ok(())
    }

    pub async fn on_stop(&self, event: EventMessage) -> Result<(), DockerStreamError> {
        let actor = event.actor.ok_or(DockerStreamError::MissingActor)?;
        let attributes = actor.attributes.ok_or(DockerStreamError::MissingAttributes)?;
        let container_name = attributes.get("name").ok_or(DockerStreamError::MissingAttributes)?;

        debug!("Container stopped: {}", container_name);

        if let Err(e) = self.metrics_listener_handle.remove_node_by_name(container_name).await {
            error!("Error removing node: {:?}", e);
        };

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DockerStreamListenerError {
    #[error("Dockerstream error: {0}")]
    DockerStreamError(#[from] DockerStreamError),

    #[error("Ivynet signing error: {0}")]
    SigningError(#[from] ivynet_signer::sign_utils::IvySigningError),

    #[error("Telemetry dispatch error: {0}")]
    DispatchError(#[from] TelemetryDispatchError),
}
