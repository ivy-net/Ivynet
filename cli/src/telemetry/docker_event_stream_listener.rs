use std::{sync::Arc, time::Duration};

use bollard::secret::{EventMessage, EventMessageTypeEnum};
use ivynet_docker::dockerapi::{DockerApi, DockerClient, DockerStreamError};
use ivynet_grpc::{
    backend::backend_client::BackendClient,
    messages::{NodeData, NodeTypeQueries, NodeTypeQuery, SignedNodeData},
    tonic::{transport::Channel, Request, Response},
};
use ivynet_node_type::NodeType;
use ivynet_signer::{sign_utils::sign_node_data, IvyWallet};
use tokio::{sync::Mutex, time::sleep};
use tokio_stream::StreamExt;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::monitor::MonitorConfig;

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
    pub merging_containers: bool,
}

impl DockerStreamListener<DockerClient> {
    pub fn new(
        metrics_listener: MetricsListenerHandle,
        logs_listener: LogsListenerManager,
        dispatch: TelemetryDispatchHandle,
        identity_wallet: IvyWallet,
        machine_id: Uuid,
        backend: BackendClient<Channel>,
        merging_containers: bool,
    ) -> Self {
        Self {
            docker: DockerClient::default(),
            metrics_listener_handle: metrics_listener,
            logs_listener_handle: logs_listener,
            dispatch,
            machine_id,
            identity_wallet,
            backend,
            merging_containers,
        }
    }

    pub async fn run(
        mut self,
        config: Arc<Mutex<MonitorConfig>>,
    ) -> Result<(), DockerStreamListenerError> {
        let mut docker_stream = self.docker.stream_events().await;
        while let Some(Ok(event)) = docker_stream.next().await {
            debug!("Dockerstream Event | {:?}", event);
            if event.typ == Some(EventMessageTypeEnum::CONTAINER) {
                if let Some(action) = event.action.as_deref() {
                    match action {
                        "start" => {
                            self.on_start(event, &config).await?;
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
        config: &Arc<Mutex<MonitorConfig>>,
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

        let metrics_port = match inc_container.metrics_port().await {
            Some(port) => Some(port),
            None => {
                // wait for metrics port to potentially come up
                sleep(Duration::from_secs(10)).await;
                inc_container.metrics_port().await
            }
        };

        // TODO: This query to backend seems to be extremely slow. Need to investigate
        let guessed_type: Option<NodeType> = if self.merging_containers {
            self.backend
                .node_type_queries(Request::new(NodeTypeQueries {
                    node_types: vec![NodeTypeQuery {
                        image_name: inc_image_name.clone(),
                        image_digest: inc_container_digest.clone(),
                        container_name: inc_container_name.to_string(),
                    }],
                }))
                .await
                .map_err(|_| DockerStreamListenerError::BackendConnectionError)?
                .into_inner()
                .node_types
                .first()
                .map(|nt| nt.node_type.as_str().into())
        } else {
            None
        };

        // We treat an avs with the same image as the same avs configuration, so whenever an avs is
        // found using the same image, we are updating the configured avs that is already on the
        // list
        let configured = match config.lock().await.activate_avs(inc_container_name, guessed_type) {
            Some(avs) => Some(avs.clone()),
            None => {
                let node_type_query = NodeTypeQuery {
                    container_name: inc_container_name.to_string(),
                    image_name: inc_image_name.clone(),
                    image_digest: inc_container_digest.clone(),
                };
                let query = Request::new(NodeTypeQueries { node_types: vec![node_type_query] });

                let node_type =
                    self.backend.node_type_queries(query).await.map(Response::into_inner).ok();

                let found_type = if let Some(node_type) = node_type {
                    node_type.node_types.first().map(|t| t.node_type.clone())
                } else {
                    None
                }
                .unwrap_or("unknown".to_string());

                Some(ConfiguredAvs {
                    assigned_name: inc_container_name.to_string(),
                    container_name: inc_container_name.to_string(),
                    image_name: Some(inc_image_name.clone()),
                    avs_type: found_type,
                    metric_port: metrics_port,
                    active: true,
                })
            }
        };

        if let Some(configured) = configured {
            debug!("Found container: {}", inc_container_name);
            info!("Found container: {}", inc_container_name);

            let node_data = NodeData {
                name: configured.container_name.clone(),
                node_type: Some(configured.avs_type.clone()),
                manifest: Some(inc_container_digest),
                metrics_alive: Some(configured.metric_port.is_some()),
                node_running: Some(true),
            };

            let node_data_signature = sign_node_data(&node_data, &self.identity_wallet)?;
            let signed_node_data = SignedNodeData {
                machine_id: self.machine_id.into(),
                signature: node_data_signature.to_vec(),
                node_data: Some(node_data),
            };

            self.dispatch.send_node_data(signed_node_data).await?;

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

    #[error("Backend connection error")]
    BackendConnectionError,
}
