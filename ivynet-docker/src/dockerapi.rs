use std::collections::HashMap;

use bollard::{
    container::LogOutput,
    image::ListImagesOptions,
    secret::{ContainerSummary, ImageSummary},
    Docker,
};
use tokio_stream::Stream;

use ivynet_node_type::NodeType;
use tracing::debug;

use super::container::Container;

#[derive(Clone)]
pub struct DockerClient(pub Docker);

pub fn connect_docker() -> Docker {
    std::env::var("DOCKER_HOST").map(|_| Docker::connect_with_defaults().unwrap()).unwrap_or_else(
        |_| {
            Docker::connect_with_local_defaults()
                .expect("Cannot connect to docker sock. Please set $DOCKER_HOST")
        },
    )
}

impl DockerClient {
    #[allow(dead_code)]
    fn new(docker: Docker) -> Self {
        Self(docker)
    }
}

impl Default for DockerClient {
    fn default() -> Self {
        Self(connect_docker())
    }
}

fn process_images(images: Vec<ImageSummary>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for image in images {
        if image.repo_digests.is_empty() {
            debug!("No repo digests on image: {:#?}", image);
            use_repo_tags(&image, &mut map);
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
                    use_repo_tags(&image, &mut map);
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

impl DockerClient {
    pub async fn list_containers(&self) -> Vec<ContainerSummary> {
        self.0.list_containers::<String>(None).await.expect("Cannot list containers")
    }

    pub async fn list_images(&self) -> HashMap<String, String> {
        let images = self
            .0
            .list_images(Some(ListImagesOptions::<String> {
                all: true,
                digests: true,
                ..Default::default()
            }))
            .await
            .expect("Cannot list images");
        process_images(images)
    }

    /// Inspect a container by container name
    pub async fn inspect_by_container_name(&self, container_name: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        let cname = container_name.to_string();
        for container in containers {
            if let Some(name) = &container.names {
                if name.contains(&cname) {
                    return Some(Container::new(container.clone()));
                }
            }
        }
        None
    }

    /// Inspect a container by image name
    pub async fn inspect(&self, image_name: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        for container in containers {
            debug!("Checking container {container:?}");
            if let Some(ref image_string) = container.image {
                if image_string.contains(image_name) {
                    return Some(Container::new(container.clone()));
                }
            }
        }
        None
    }

    /// Inspect multiple containers by image name. Returns a vector of found containers.
    pub async fn inspect_many(&self, image_names: &[&str]) -> Vec<Container> {
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

    pub async fn find_container_by_name(&self, name: &str) -> Option<Container> {
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
    pub async fn find_node_container(&self, node_type: &NodeType) -> Option<Container> {
        let image_name = node_type.default_repository().unwrap();
        self.inspect(image_name).await
    }

    /// Find all active containers for a slice of node types
    pub async fn find_node_containers(&self, node_types: &[NodeType]) -> Vec<Container> {
        let image_names: Vec<&str> =
            node_types.iter().map(|node_type| node_type.default_repository().unwrap()).collect();
        self.inspect_many(&image_names).await
    }

    /// Find all active containers for all available node types
    pub async fn find_all_node_containers(&self) -> Vec<Container> {
        let node_types = NodeType::all_known_with_repo();
        self.find_node_containers(&node_types).await
    }

    /// Stream logs for a given node type since a given timestamp
    pub async fn stream_logs_for_node(
        &self,
        node_type: &NodeType,
        since: i64,
    ) -> Option<impl Stream<Item = Result<LogOutput, bollard::errors::Error>>> {
        let container = self.find_node_container(node_type).await;
        container.map(|container| container.stream_logs(self, since))
    }

    /// Stream logs for a given node type since current time
    pub async fn stream_logs_for_node_latest(
        &self,
        node_type: &NodeType,
    ) -> Option<impl Stream<Item = Result<LogOutput, bollard::errors::Error>>> {
        let container = self.find_node_container(node_type).await;
        container.map(|container| container.stream_logs_latest(self))
    }

    /// Stream logs for all nodes since a given timestamp. Returns a merged stream.
    pub async fn stream_logs_for_all_nodes(
        &self,
        since: i64,
    ) -> impl Stream<Item = Result<LogOutput, bollard::errors::Error>> {
        let containers = self.find_all_node_containers().await;
        let streams = containers.into_iter().map(|container| container.stream_logs(self, since));
        futures::stream::select_all(streams)
    }

    /// Stream logs for all nodes since current time. Returns a merged stream.
    pub async fn stream_logs_for_all_nodes_latest(
        &self,
    ) -> impl Stream<Item = Result<LogOutput, bollard::errors::Error>> {
        let containers = self.find_all_node_containers().await;
        let streams = containers.into_iter().map(|container| container.stream_logs_latest(self));
        futures::stream::select_all(streams)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bollard::secret::ImageSummary;
    use std::sync::Arc;

    // Define trait for Docker behavior we want to mock
    #[async_trait]
    trait DockerImages {
        async fn list_images(
            &self,
            options: Option<ListImagesOptions<String>>,
        ) -> Result<Vec<ImageSummary>, bollard::errors::Error>;
    }

    // Implement for real Docker
    #[async_trait]
    impl DockerImages for Docker {
        async fn list_images(
            &self,
            options: Option<ListImagesOptions<String>>,
        ) -> Result<Vec<ImageSummary>, bollard::errors::Error> {
            self.list_images(options).await
        }
    }

    // Mock implementation
    struct MockDocker {
        images: Vec<ImageSummary>,
        should_fail: bool,
        error_kind: Option<std::io::ErrorKind>,
    }

    impl MockDocker {
        fn with_images(images: Vec<ImageSummary>) -> Self {
            Self { images, should_fail: false, error_kind: None }
        }

        fn with_error(error_kind: std::io::ErrorKind) -> Self {
            Self { images: Vec::new(), should_fail: true, error_kind: Some(error_kind) }
        }
    }

    #[async_trait]
    impl DockerImages for MockDocker {
        async fn list_images(
            &self,
            _options: Option<ListImagesOptions<String>>,
        ) -> Result<Vec<ImageSummary>, bollard::errors::Error> {
            if self.should_fail {
                if let Some(kind) = self.error_kind {
                    return Err(bollard::errors::Error::from(std::io::Error::new(
                        kind,
                        "Docker API error",
                    )));
                }
            }
            Ok(self.images.clone())
        }
    }

    // Modified DockerClient to accept any type implementing DockerImages
    struct TestDockerClient<T: DockerImages> {
        docker: Arc<T>,
    }

    impl<T: DockerImages> TestDockerClient<T> {
        fn new(docker: T) -> Self {
            Self { docker: Arc::new(docker) }
        }

        async fn list_images(&self) -> HashMap<String, String> {
            let images = self
                .docker
                .list_images(Some(ListImagesOptions::<String> {
                    all: true,
                    digests: true,
                    ..Default::default()
                }))
                .await
                .expect("Cannot list images");
            process_images(images)
        }
    }

    #[tokio::test]
    async fn test_list_images_normal_case() {
        let mock = MockDocker::with_images(vec![ImageSummary {
            id: "sha256:digest1".to_string(),
            repo_tags: vec!["image:latest".to_string()],
            repo_digests: vec!["image@sha256:digest1".to_string()],
            ..Default::default()
        }]);

        let client = TestDockerClient::new(mock);
        let result = client.list_images().await;

        assert_eq!(result.get("image:latest").unwrap(), "sha256:digest1");
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_list_images_empty_repo_tags() {
        let mock = MockDocker::with_images(vec![ImageSummary {
            id: "sha256:digest1".to_string(),
            repo_tags: vec![],
            repo_digests: vec!["image1@sha256:digest1".to_string()],
            ..Default::default()
        }]);

        let client = TestDockerClient::new(mock);
        let result = client.list_images().await;

        assert_eq!(result.get("image1").unwrap(), "sha256:digest1");
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_list_images_empty_repo_digests() {
        let mock = MockDocker::with_images(vec![ImageSummary {
            id: "sha256:digest1".to_string(),
            repo_tags: vec!["image:latest".to_string()],
            repo_digests: vec![],
            ..Default::default()
        }]);

        let client = TestDockerClient::new(mock);
        let result = client.list_images().await;

        assert_eq!(result.get("image:latest").unwrap(), "sha256:digest1");
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot list images")]
    async fn test_list_images_error() {
        let mock = MockDocker::with_error(std::io::ErrorKind::ConnectionRefused);
        let client = TestDockerClient::new(mock);
        client.list_images().await;
    }

    #[tokio::test]
    async fn test_list_images_multiple_tags() {
        let mock = MockDocker::with_images(vec![ImageSummary {
            id: "sha256:digest4".to_string(),
            repo_tags: vec!["image:latest".to_string(), "image:v1".to_string()],
            repo_digests: vec!["image@sha256:digest4".to_string()],
            ..Default::default()
        }]);

        let client = TestDockerClient::new(mock);
        let result = client.list_images().await;

        assert_eq!(result.get("image:latest").unwrap(), "sha256:digest4");
        assert_eq!(result.get("image:v1").unwrap(), "sha256:digest4");
        assert_eq!(result.len(), 2);
    }
}
