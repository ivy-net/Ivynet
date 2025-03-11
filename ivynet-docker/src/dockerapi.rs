use std::{collections::HashMap, pin::Pin};

use async_trait::async_trait;
use bollard::{
    container::{LogOutput, LogsOptions},
    errors::Error,
    secret::{EventMessage, ImageSummary},
    Docker,
};
use futures::future::join_all;
use tokio_stream::Stream;
use tracing::debug;

use crate::container::ContainerId;

use super::container::{Container, ContainerImage};

#[derive(Debug, Clone)]
pub struct DockerClient(pub Docker);

pub fn connect_docker() -> Docker {
    std::env::var("DOCKER_HOST").map(|_| Docker::connect_with_defaults().unwrap()).unwrap_or_else(
        |_| {
            Docker::connect_with_local_defaults()
                .expect("Cannot connect to docker sock. Please set $DOCKER_HOST")
        },
    )
}

impl Default for DockerClient {
    fn default() -> Self {
        Self(connect_docker())
    }
}

// TODO: Implement lifetimes to allow passing as ref
#[async_trait]
pub trait DockerApi: Clone + Sync + Send + 'static {
    async fn list_containers(&self) -> Vec<Container>;
    fn inner(&self) -> Docker;

    async fn stream_logs(
        &self,
        container: Container,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>>;

    async fn stream_events(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<EventMessage, Error>> + Send + Unpin>>;

    async fn stream_logs_by_container_id(
        &self,
        container_id: &str,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>>;

    async fn inspect(&self, image_name: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        for container in containers {
            println!("Checking container {container:?}");
            if let Some(image_string) = container.image() {
                if image_string.contains(image_name) {
                    return Some(container);
                }
            }
        }
        None
    }

    /// Checks if a container is running by container name
    async fn is_running(&self, container_name: &str) -> bool {
        if let Some(container) = self.find_container_by_name(container_name).await {
            if let Some(state) = container.state() {
                return state.to_lowercase() == "running";
            }
        }

        false
    }

    /// Inspect multiple containers by image name. Returns a vector of found containers.
    async fn inspect_many(&self, image_names: &[&str]) -> Vec<Container> {
        let containers = self.list_containers().await;
        containers
            .into_iter()
            .filter(|container| {
                container
                    .image()
                    .as_ref()
                    .map(|image_string| image_names.iter().any(|name| image_string.contains(name)))
                    .unwrap_or_default()
            })
            .collect()
    }

    async fn find_container_by_name(&self, container_name: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        containers.into_iter().find(|container| {
            container
                .names()
                .as_ref()
                .map(|names| names.iter().any(|n| n.contains(container_name)))
                .unwrap_or_default()
        })
    }

    async fn find_containers_by_name(&self, container_names: &[&str]) -> Vec<Container> {
        let containers = self.list_containers().await;
        containers
            .into_iter()
            .filter(|container| {
                container
                    .names()
                    .as_ref()
                    .map(|names| {
                        names.iter().any(|n| container_names.iter().any(|cn| n.contains(cn)))
                    })
                    .unwrap_or_default()
            })
            .collect()
    }

    async fn find_container_by_image_id(&self, digest: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        for container in containers {
            if let Some(image_id) = container.repo_digest(&self.inner()).await {
                if image_id == digest {
                    return Some(container);
                }
            }
        }
        None
    }

    async fn find_container_by_image(&self, image: &str, strict: bool) -> Option<Container> {
        let containers = self.list_containers().await;
        for container in containers {
            if let Some(image_id) = container.repo_digest(&self.inner()).await {
                if strict {
                    if ContainerImage::from(image_id.as_str()) == ContainerImage::from(image) {
                        return Some(container);
                    }
                } else if ContainerImage::from(image_id.as_str()).repository ==
                    ContainerImage::from(image).repository
                {
                    return Some(container);
                }
            }
        }
        None
    }

    async fn stream_logs_latest(
        &self,
        container: Container,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let now = chrono::Utc::now().timestamp();
        self.stream_logs(container, now).await
    }

    /// Stream logs for a given node type since a given timestamp
    async fn stream_logs_for_node(
        &self,
        node_type: &str,
        since: i64,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>> {
        let container = self.find_container_by_name(node_type).await?;
        Some(self.stream_logs(container, since).await)
    }

    /// Stream logs for a given node type since current time
    async fn stream_logs_for_node_latest(
        &self,
        node_type: &str,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>> {
        let container = self.find_container_by_name(node_type).await?;
        Some(self.stream_logs_latest(container).await)
    }

    /// Stream logs for all nodes since a given timestamp. Returns a merged stream.
    async fn stream_logs_for_nodes(
        &self,
        nodes: &[&str],
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let containers = self.find_containers_by_name(nodes).await;
        let stream_futures =
            containers.into_iter().map(|container| self.stream_logs(container, since));
        let streams = join_all(stream_futures).await;
        Box::pin(futures::stream::select_all(streams))
    }

    /// Stream logs for all nodes since current time. Returns a merged stream.
    async fn stream_logs_for_all_nodes_latest(
        &self,
        nodes: &[&str],
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let containers = self.find_containers_by_name(nodes).await;
        let stream_futures =
            containers.into_iter().map(|container| self.stream_logs_latest(container));
        let streams = join_all(stream_futures).await;
        Box::pin(futures::stream::select_all(streams))
    }

    fn use_repo_tags(image: &ImageSummary, map: &mut HashMap<ContainerId, ContainerImage>) {
        debug!("REPO DIGESTS BROKEN: {:#?}", image);
        debug!("Using repo_tags instead");
        for tag in &image.repo_tags {
            map.insert(
                ContainerId::from(image.id.clone().as_str()),
                ContainerImage::from(tag.as_str()),
            );
        }
    }
}

#[async_trait]
impl DockerApi for DockerClient {
    async fn list_containers(&self) -> Vec<Container> {
        let containers =
            self.0.list_containers::<String>(None).await.expect("Cannot list containers");
        containers.into_iter().map(Container::new).collect()
    }

    fn inner(&self) -> Docker {
        self.0.clone()
    }

    async fn stream_logs(
        &self,
        container: Container,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let log_opts: LogsOptions<&str> =
            LogsOptions { follow: true, stdout: true, stderr: true, since, ..Default::default() };
        Box::pin(self.0.logs(container.id().unwrap(), Some(log_opts)))
    }

    async fn stream_logs_by_container_id(
        &self,
        container_id: &str,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let log_opts: LogsOptions<&str> =
            LogsOptions { follow: true, stdout: true, stderr: true, since, ..Default::default() };
        Box::pin(self.0.logs(container_id, Some(log_opts)))
    }

    async fn stream_events(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<EventMessage, Error>> + Send + Unpin>> {
        Box::pin(self.0.events::<&str>(None))
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum DockerStreamError {
    #[error("Dockerstream is missing the actor field")]
    MissingActor,

    #[error("Dockerstream is missing the attributes field")]
    MissingAttributes,

    #[error("Dockerstream image name mismatch: {0} != {1}")]
    ImageNameMismatch(String, String),
}

#[cfg(test)]
mod dockerapi_tests {}
