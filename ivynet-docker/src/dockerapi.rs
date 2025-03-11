use std::{collections::HashMap, pin::Pin, str::FromStr};

use async_trait::async_trait;
use bollard::{
    container::{LogOutput, LogsOptions},
    errors::Error,
    image::ListImagesOptions,
    secret::{ContainerInspectResponse, EventMessage, ImageSummary},
    Docker,
};
use futures::future::join_all;
use tokio_stream::Stream;
use tracing::debug;

use crate::{
    container::{ContainerId, FullContainer},
    repodigest::RepoTag,
};

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
    async fn get_container_by_name(
        &self,
        container_name: &str,
    ) -> Result<ContainerInspectResponse, DockerClientError>;

    async fn get_full_container_by_name(
        &self,
        container_name: &str,
    ) -> Result<FullContainer, DockerClientError> {
        let container = self.get_container_by_name(container_name).await?;
        let image_name = container
            .clone()
            .config
            .ok_or(DockerClientError::ContainerNotFound(container_name.to_string()))?
            .image
            .ok_or(DockerClientError::ImageNotFoundForContainer(container_name.to_string()))?;
        let image_inspect = self.inner().inspect_image(&image_name).await?;
        Ok(FullContainer::new(container, image_inspect))
    }

    async fn list_containers(&self) -> Vec<FullContainer>;
    fn inner(&self) -> Docker;

    async fn stream_logs(
        &self,
        container: FullContainer,
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

    // async fn inspect(&self, image_name: &str) -> Option<FullContainer> {
    //     let containers = self.list_containers().await;
    //     for container in containers {
    //         println!("Checking container {container:?}");
    //         if let Some(image_string) = container.image() {
    //             if image_string.contains(image_name) {
    //                 return Some(container);
    //             }
    //         }
    //     }
    //     None
    // }

    /// Checks if a container is running by container name
    async fn is_running(&self, container_name: &str) -> bool {
        if let Ok(container) = self.get_full_container_by_name(container_name).await {
            if let Some(state) = container.state() {
                if let Some(status) = state.status {
                    return status.to_string().to_lowercase() == "running";
                }
            }
        }

        false
    }

    async fn get_containers_by_name(&self, container_names: &[&str]) -> Vec<FullContainer> {
        let mut containers = Vec::new();
        for name in container_names {
            if let Ok(container) = self.get_full_container_by_name(name).await {
                containers.push(container);
            }
        }
        containers
    }

    async fn find_container_by_image_id(&self, digest: &str) -> Option<FullContainer> {
        let containers = self.list_containers().await;
        containers.into_iter().find(|container| container.image_id() == Some(digest))
    }

    async fn find_container_by_image(&self, image: &str, strict: bool) -> Option<FullContainer> {
        let containers = self.list_containers().await;
        containers.into_iter().find(|container| {
            if let Some(image_id) = container.image_id() {
                if strict {
                    RepoTag::from_str(image_id) == RepoTag::from_str(image)
                } else {
                    RepoTag::from_str(image_id).expect("unenterable") ==
                        RepoTag::from_str(image).expect("unenterable")
                }
            } else {
                false
            }
        })
    }

    async fn stream_logs_latest(
        &self,
        container: FullContainer,
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
        let container = self.get_full_container_by_name(node_type).await.ok()?;
        Some(self.stream_logs(container, since).await)
    }

    /// Stream logs for a given node type since current time
    async fn stream_logs_for_node_latest(
        &self,
        node_type: &str,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send>>> {
        let container = self.get_full_container_by_name(node_type).await.ok()?;
        Some(self.stream_logs_latest(container).await)
    }

    /// Stream logs for all nodes since a given timestamp. Returns a merged stream.
    async fn stream_logs_for_nodes(
        &self,
        nodes: &[&str],
        since: i64,
    ) -> Pin<Box<dyn Stream<Item = Result<LogOutput, Error>> + Send + Unpin>> {
        let containers = self.get_containers_by_name(nodes).await;
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
        let containers = self.get_containers_by_name(nodes).await;
        let stream_futures =
            containers.into_iter().map(|container| self.stream_logs_latest(container));
        let streams = join_all(stream_futures).await;
        Box::pin(futures::stream::select_all(streams))
    }

    fn process_images(images: Vec<ImageSummary>) -> HashMap<ContainerId, RepoTag> {
        let mut map = HashMap::new();
        for image in images {
            if image.repo_digests.is_empty() {
                debug!("No repo digests on image: {:#?}", image);
                DockerClient::use_repo_tags(&image, &mut map);
            } else if image.repo_tags.is_empty() && image.repo_digests.is_empty() {
                debug!("No repo tags or digests on image: {:#?}", image);
                map.insert(
                    ContainerId::from(image.id.clone().as_str()),
                    RepoTag::from_str("local")
                        .expect("Cannot parse repo tag, this should be unenterable."),
                );
            } else {
                for digest in &image.repo_digests {
                    let elements = digest.split("@").collect::<Vec<_>>();
                    if elements.len() == 2 {
                        if !image.repo_tags.is_empty() {
                            for tag in &image.repo_tags {
                                map.insert(
                                    ContainerId::from(elements[1]),
                                    RepoTag::from_str(tag.as_str()).expect(
                                        "Cannot parse repo tag, this should be unenterable.",
                                    ),
                                );
                            }
                        } else {
                            debug!("No repo tags on image: {}", elements[0]);
                            debug!("This should get a debug later as well in node_type");
                            map.insert(
                                ContainerId::from(elements[1]),
                                RepoTag::from_str(elements[0])
                                    .expect("Cannot parse repo tag, this should be unenterable."),
                            );
                        }
                    } else {
                        DockerClient::use_repo_tags(&image, &mut map);
                    }
                }
            }
        }
        map
    }

    fn use_repo_tags(image: &ImageSummary, map: &mut HashMap<ContainerId, RepoTag>) {
        debug!("REPO DIGESTS BROKEN: {:#?}", image);
        debug!("Using repo_tags instead");
        for tag in &image.repo_tags {
            map.insert(
                ContainerId::from(image.id.clone().as_str()),
                RepoTag::from_str(tag.as_str())
                    .expect("Cannot parse repo tag, this should be unenterable."),
            );
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DockerClientError {
    #[error("Container not found: {0}")]
    ContainerNotFound(String),
    #[error("Image not found: {0}")]
    ImageNotFoundForContainer(String),
    #[error(transparent)]
    DockerError(#[from] Error),
}

#[async_trait]
impl DockerApi for DockerClient {
    async fn get_container_by_name(
        &self,
        container_name: &str,
    ) -> Result<ContainerInspectResponse, DockerClientError> {
        Ok(self.0.inspect_container(container_name, None).await?)
    }

    async fn list_containers(&self) -> Vec<FullContainer> {
        let containers =
            self.0.list_containers::<String>(None).await.expect("Cannot list containers");
        let names: Vec<String> = containers
            .iter()
            .filter_map(|c| c.names.clone())
            .collect::<Vec<_>>()
            .into_iter()
            .flatten()
            .map(|s| s.trim_start_matches("/").to_string())
            .collect();
        let mut containers = Vec::new();
        for n in names {
            if let Ok(container) = self.get_full_container_by_name(&n).await {
                containers.push(container);
            }
        }
        containers
    }

    fn inner(&self) -> Docker {
        self.0.clone()
    }

    async fn stream_logs(
        &self,
        container: FullContainer,
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
