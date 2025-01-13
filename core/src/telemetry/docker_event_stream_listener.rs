use std::{collections::HashMap, time::Duration};

use bollard::secret::{EventMessage, EventMessageTypeEnum};
use ivynet_docker::{
    dockerapi::{DockerApi, DockerClient, DockerStreamError},
    get_node_type,
};
use ivynet_node_type::NodeType;
use tokio::time::sleep;
use tokio_stream::StreamExt;
use tonic::{transport::Channel, Request, Response};
use tracing::{debug, error};
use uuid::Uuid;

use crate::grpc::{
    backend::backend_client::BackendClient,
    messages::{Digests, NodeTypes},
};

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
        let container_name = attributes.get("name").ok_or(DockerStreamError::MissingAttributes)?;

        let container = match self.docker.find_container_by_name(container_name).await {
            Some(container) => container,
            None => {
                return Ok(());
            }
        };

        let image_name = container.image().unwrap_or_default().to_string();
        let container_digest = container.image_id().unwrap_or_default().to_string();

        let metrics_port = match container.metrics_port().await {
            Some(port) => Some(port),
            None => {
                // wait for metrics port to potentially come up
                sleep(Duration::from_secs(10)).await;
                container.metrics_port().await
            }
        };

        let configured = match avses.iter().find(|avs| avs.container_name == *container_name) {
            Some(avs) => Some(avs.clone()),
            None => {
                // get node type
                // This is mostly copy-pasted from `src/monitor.rs:143`, should abstract to a
                // common method

                // TODO: GRPC method for fetching a single node type from a digest instead of this.

                let node_types: Option<NodeTypes> = self
                    .backend
                    .node_types(Request::new(Digests { digests: vec![container_digest.clone()] }))
                    .await
                    .map(Response::into_inner)
                    .ok();

                let hashes: HashMap<String, NodeType> = match node_types {
                    Some(types) => types
                        .node_types
                        .into_iter()
                        .map(|nt| (nt.digest, NodeType::from(nt.node_type.as_str())))
                        .collect::<HashMap<_, _>>(),
                    None => HashMap::new(),
                };

                let node_type =
                    get_node_type(&hashes, &container_digest, &image_name, container_name);
                node_type.map(|node_type| ConfiguredAvs {
                    assigned_name: format!("{}-{}", container_name, Uuid::new_v4()),
                    container_name: container_name.clone(),
                    avs_type: node_type,
                    metric_port: metrics_port,
                })
            }
        };

        if let Some(configured) = configured {
            debug!("Found container: {}", container_name);

            if let Err(e) = self.metrics_listener_handle.add_node(&configured).await {
                error!("Error adding node: {:?}", e);
            }
            if let Err(e) = self.logs_listener_handle.add_listener(&container, &configured).await {
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
