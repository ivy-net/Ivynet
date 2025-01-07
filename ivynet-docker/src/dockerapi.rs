use std::{collections::HashMap, pin::Pin};

use async_trait::async_trait;
use bollard::{
    container::{LogOutput, LogsOptions},
    errors::Error,
    image::ListImagesOptions,
    secret::{ContainerSummary, EventMessage},
    Docker,
};
use futures::StreamExt;
use tokio_stream::Stream;

use ivynet_node_type::NodeType;

use super::container::Container;

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
#[async_trait]
pub trait DockerApi {
    async fn list_containers(&self) -> Vec<ContainerSummary>;

    async fn list_images(&self) -> HashMap<String, String>;

    async fn inspect(&self, image_name: &str) -> Option<Container>;

    async fn inspect_many(&self, image_names: &[&str]) -> Vec<Container>;

    async fn find_container_by_name(&self, name: &str) -> Option<Container>;

    async fn find_node_container(&self, node_type: &NodeType) -> Option<Container>;

    async fn find_node_containers(&self, node_types: &[NodeType]) -> Vec<Container>;

    async fn find_all_node_containers(&self) -> Vec<Container>;

    async fn stream_logs(
        &self,
        container: &Container,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>>;

    async fn stream_logs_for_node(
        &self,
        node_type: &NodeType,
        since: i64,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>>;

    async fn stream_logs_for_node_latest(
        &self,
        node_type: &NodeType,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>>;

    async fn stream_logs_for_all_nodes(
        &self,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>>;

    async fn stream_logs_for_all_nodes_latest(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>>;

    async fn stream_events(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<EventMessage, Error>> + Send + Unpin>>;
}

#[async_trait]
impl DockerApi for DockerClient {
    async fn list_containers(&self) -> Vec<ContainerSummary> {
        self.0.list_containers::<String>(None).await.expect("Cannot list containers")
    }

    async fn list_images(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for image in self
            .0
            .list_images(Some(ListImagesOptions::<String> {
                all: true,
                digests: true,
                ..Default::default()
            }))
            .await
            .expect("Cannot list images")
        {
            for digest in &image.repo_digests {
                let elements = digest.split("@").collect::<Vec<_>>();
                if elements.len() == 2 {
                    for tag in &image.repo_tags {
                        map.insert(tag.clone(), elements[1].to_string());
                    }
                }
            }
        }
        map
    }

    /// Inspect a container by image name
    async fn inspect(&self, image_name: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        for container in containers {
            println!("Checking container {container:?}");
            if let Some(ref image_string) = container.image {
                if image_string.contains(image_name) {
                    return Some(Container::new(container.clone()));
                }
            }
        }
        None
    }

    /// Inspect multiple containers by image name. Returns a vector of found containers.
    async fn inspect_many(&self, image_names: &[&str]) -> Vec<Container> {
        let containers = self.list_containers().await;
        containers
            .into_iter()
            .filter(|container| {
                container
                    .image
                    .as_ref()
                    .map(|image_string| image_names.iter().any(|name| image_string.contains(name)))
                    .unwrap_or_default()
            })
            .map(Container::new)
            .collect()
    }

    async fn find_container_by_name(&self, name: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        containers
            .into_iter()
            .find(|container| {
                container
                    .names
                    .as_ref()
                    .map(|names| names.iter().any(|n| n.contains(name)))
                    .unwrap_or_default()
            })
            .map(Container::new)
    }

    /// Find an active container for a given node type
    async fn find_node_container(&self, node_type: &NodeType) -> Option<Container> {
        let image_name = node_type.default_repository().unwrap();
        self.inspect(image_name).await
    }

    /// Find all active containers for a slice of node types
    async fn find_node_containers(&self, node_types: &[NodeType]) -> Vec<Container> {
        let image_names: Vec<&str> =
            node_types.iter().map(|node_type| node_type.default_repository().unwrap()).collect();
        self.inspect_many(&image_names).await
    }

    /// Find all active containers for all available node types
    async fn find_all_node_containers(&self) -> Vec<Container> {
        let node_types = NodeType::all_known();
        self.find_node_containers(&node_types).await
    }

    async fn stream_logs(
        &self,
        container: &Container,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let log_opts: LogsOptions<&str> =
            LogsOptions { follow: true, stdout: true, stderr: true, since, ..Default::default() };
        Box::pin(self.0.logs(container.id().unwrap(), Some(log_opts)))
    }

    /// Stream logs for a given node type since a given timestamp
    async fn stream_logs_for_node(
        &self,
        node_type: &NodeType,
        since: i64,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>> {
        let container = self.find_node_container(node_type).await;
        container.map(|container| container.stream_logs(self, since).boxed())
    }

    /// Stream logs for a given node type since current time
    async fn stream_logs_for_node_latest(
        &self,
        node_type: &NodeType,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>> {
        let container = self.find_node_container(node_type).await;
        container.map(|container| container.stream_logs_latest(self).boxed())
    }

    /// Stream logs for all nodes since a given timestamp. Returns a merged stream.
    async fn stream_logs_for_all_nodes(
        &self,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let containers = self.find_all_node_containers().await;
        let streams = containers.into_iter().map(|container| container.stream_logs(self, since));
        Box::pin(futures::stream::select_all(streams))
    }

    /// Stream logs for all nodes since current time. Returns a merged stream.
    async fn stream_logs_for_all_nodes_latest(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let containers = self.find_all_node_containers().await;
        let streams = containers.into_iter().map(|container| container.stream_logs_latest(self));
        Box::pin(futures::stream::select_all(streams))
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
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_images() {
        let client = super::DockerClient::default();
        // let containers = client.list_images().await;
        // println!("{:?}", containers);
    }
}
