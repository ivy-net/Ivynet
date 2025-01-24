use std::{collections::HashMap, pin::Pin};

use async_trait::async_trait;
use bollard::{
    container::{LogOutput, LogsOptions},
    errors::Error,
    image::ListImagesOptions,
    secret::{ContainerSummary, EventMessage, ImageSummary},
    Docker,
};
use futures::future::join_all;
use tokio_stream::Stream;
use tracing::debug;

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

// TODO: Implement lifetimes to allow passing as ref
#[async_trait]
pub trait DockerApi: Clone + Sync + Send + 'static {
    async fn list_containers(&self) -> Vec<ContainerSummary>;
    async fn list_images(&self) -> HashMap<String, String>;

    async fn stream_logs(
        &self,
        container: Container,
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>>;

    async fn stream_events(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<EventMessage, Error>> + Send + Unpin>>;

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
                    .image
                    .as_ref()
                    .map(|image_string| image_names.iter().any(|name| image_string.contains(name)))
                    .unwrap_or_default()
            })
            .map(Container::new)
            .collect()
    }

    async fn find_container_by_name(&self, container_name: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        containers
            .into_iter()
            .find(|container| {
                container
                    .names
                    .as_ref()
                    .map(|names| names.iter().any(|n| n.contains(container_name)))
                    .unwrap_or_default()
            })
            .map(Container::new)
    }

    async fn find_containers_by_name(&self, container_names: &[&str]) -> Vec<Container> {
        let containers = self.list_containers().await;
        containers
            .into_iter()
            .filter(|container| {
                container
                    .names
                    .as_ref()
                    .map(|names| {
                        names.iter().any(|n| container_names.iter().any(|cn| n.contains(cn)))
                    })
                    .unwrap_or_default()
            })
            .map(Container::new)
            .collect()
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

    fn process_images(images: Vec<ImageSummary>) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for image in images {
            if image.repo_digests.is_empty() {
                debug!("No repo digests on image: {:#?}", image);
                DockerClient::use_repo_tags(&image, &mut map);
            } else if image.repo_tags.is_empty() && image.repo_digests.is_empty() {
                debug!("No repo tags or digests on image: {:#?}", image);
                map.insert("local".to_string(), image.id.clone());
            } else {
                for digest in &image.repo_digests {
                    let elements = digest.split("@").collect::<Vec<_>>();
                    if elements.len() == 2 {
                        if !image.repo_tags.is_empty() {
                            for tag in &image.repo_tags {
                                map.insert(tag.clone(), elements[1].to_string());
                            }
                        } else {
                            debug!("No repo tags on image: {}", elements[0]);
                            debug!("This should get a debug later as well in node_type");
                            map.insert(elements[0].to_string(), elements[1].to_string());
                        }
                    } else {
                        DockerClient::use_repo_tags(&image, &mut map);
                    }
                }
            }
        }
        map
    }

    fn use_repo_tags(image: &ImageSummary, map: &mut HashMap<String, String>) {
        debug!("REPO DIGESTS BROKEN: {:#?}", image);
        debug!("Using repo_tags instead");
        for tag in &image.repo_tags {
            map.insert(tag.clone(), image.id.clone());
        }
    }
}

#[async_trait]
impl DockerApi for DockerClient {
    async fn list_containers(&self) -> Vec<ContainerSummary> {
        self.0.list_containers::<String>(None).await.expect("Cannot list containers")
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

    async fn stream_events(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<EventMessage, Error>> + Send + Unpin>> {
        Box::pin(self.0.events::<&str>(None))
    }

    async fn list_images(&self) -> HashMap<String, String> {
        let images = self
            .0
            .list_images(Some(ListImagesOptions::<String> {
                all: true,
                digests: true,
                ..Default::default()
            }))
            .await
            .expect("Cannot list images");
        DockerClient::process_images(images)
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
