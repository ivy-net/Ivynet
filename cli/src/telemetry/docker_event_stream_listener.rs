use std::time::Duration;

use bollard::secret::{EventMessage, EventMessageTypeEnum};
use ivynet_docker::dockerapi::{DockerApi, DockerClient, DockerStreamError};
use ivynet_grpc::{
    backend::backend_client::BackendClient,
    messages::NodeTypeQuery,
    tonic::{transport::Channel, Request, Response},
};
use tokio::time::sleep;
use tokio_stream::StreamExt;
use tracing::{debug, error};
use uuid::Uuid;

use super::{
    logs_listener::LogsListenerManager, metrics_listener::MetricsListenerHandle, ConfiguredAvs,
};

#[derive(Debug)]
pub struct DockerStreamListener<D: DockerApi> {
    pub docker: D,
    pub metrics_listener_handle: MetricsListenerHandle,
    pub logs_listener_handle: LogsListenerManager,
    pub backend: BackendClient<Channel>,
}

impl DockerStreamListener<DockerClient> {
    pub fn new(
        metrics_listener: MetricsListenerHandle,
        logs_listener: LogsListenerManager,
        backend: BackendClient<Channel>,
    ) -> Self {
        Self {
            docker: DockerClient::default(),
            metrics_listener_handle: metrics_listener,
            logs_listener_handle: logs_listener,
            backend,
        }
    }

    pub async fn run(mut self, known_nodes: Vec<ConfiguredAvs>) -> Result<(), DockerStreamError> {
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
    ) -> Result<(), DockerStreamError> {
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

        let configured = match avses.iter().find(|avs| avs.container_name == *inc_container_name) {
            Some(avs) => Some(avs.clone()),
            None => {
                let node_type_query = NodeTypeQuery {
                    container_name: inc_container_name.clone(),
                    image_name: inc_image_name.clone(),
                    image_digest: inc_container_digest.clone(),
                };

                let node_type = self
                    .backend
                    .node_type_query(Request::new(node_type_query))
                    .await
                    .map(Response::into_inner)
                    .ok();

                node_type.map(|node_type| ConfiguredAvs {
                    assigned_name: format!("{}-{}", inc_container_name, Uuid::new_v4()),
                    container_name: inc_container_name.clone(),
                    avs_type: node_type.node_type,
                    metric_port: metrics_port,
                })
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

pub trait ToConfiguredAvs {
    fn to_configured_avs(&self) -> Option<ConfiguredAvs>;
}
