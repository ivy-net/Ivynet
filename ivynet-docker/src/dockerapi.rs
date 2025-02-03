use std::{collections::HashMap, fmt::Display, pin::Pin};

use async_trait::async_trait;
use bollard::{
    container::{LogOutput, LogsOptions},
    errors::Error,
    image::ListImagesOptions,
    secret::{EventMessage, ImageSummary},
    Docker,
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio_stream::Stream;
use tracing::debug;

use super::container::Container;

#[derive(Debug, Clone)]
pub struct DockerClient(pub Docker);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sha256Hash([u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct DockerImage {
    pub image: String,
    pub version_tag: Option<String>,
}

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

impl From<[u8; 32]> for Sha256Hash {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl Sha256Hash {
    pub fn from_string(value: &str) -> Self {
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() != 2 || parts[0] != "sha256" {
            panic!("Invalid SHA256 hash format");
        }

        let hash = parts[1];
        if hash.len() != 64 {
            panic!("Invalid hash length");
        }

        let mut hash_bytes = [0u8; 32];
        for i in 0..32 {
            let byte_str = &hash[i * 2..i * 2 + 2];
            hash_bytes[i] = u8::from_str_radix(byte_str, 16).expect("Invalid hex character");
        }

        Self(hash_bytes)
    }
}

impl Display for Sha256Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const HEX_CHARS: [u8; 16] = *b"0123456789abcdef";
        let mut result = String::with_capacity(71); // "sha256:" + 64 hex chars
        result.push_str("sha256:");

        for &byte in &self.0 {
            result.push(HEX_CHARS[(byte >> 4) as usize] as char);
            result.push(HEX_CHARS[(byte & 0xf) as usize] as char);
        }

        f.write_str(&result)
    }
}

impl From<DockerImage> for String {
    fn from(value: DockerImage) -> Self {
        if let Some(version_tag) = value.version_tag {
            format!("{}:{}", value.image, version_tag)
        } else {
            value.image
        }
    }
}

impl From<&str> for DockerImage {
    fn from(value: &str) -> Self {
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() == 2 {
            Self { image: parts[0].to_string(), version_tag: Some(parts[1].to_string()) }
        } else {
            Self { image: value.to_string(), version_tag: None }
        }
    }
}

// TODO: Implement lifetimes to allow passing as ref
#[async_trait]
pub trait DockerApi: Clone + Sync + Send + 'static {
    async fn list_containers(&self) -> Vec<Container>;
    async fn list_images(&self) -> HashMap<Sha256Hash, DockerImage>;
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

    async fn find_container_by_id(&self, id: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        containers.into_iter().find(|container| container.id() == Some(id))
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

    fn process_images(images: Vec<ImageSummary>) -> HashMap<Sha256Hash, DockerImage> {
        let mut map = HashMap::new();
        for image in images {
            if image.repo_digests.is_empty() {
                debug!("No repo digests on image: {:#?}", image);
                DockerClient::use_repo_tags(&image, &mut map);
            } else if image.repo_tags.is_empty() && image.repo_digests.is_empty() {
                debug!("No repo tags or digests on image: {:#?}", image);
                map.insert(Sha256Hash::from_string(&image.id.clone()), DockerImage::from("local"));
            } else {
                for digest in &image.repo_digests {
                    let elements = digest.split("@").collect::<Vec<_>>();
                    if elements.len() == 2 {
                        if !image.repo_tags.is_empty() {
                            for tag in &image.repo_tags {
                                map.insert(
                                    Sha256Hash::from_string(elements[1]),
                                    DockerImage::from(tag.as_str()),
                                );
                            }
                        } else {
                            debug!("No repo tags on image: {}", elements[0]);
                            debug!("This should get a debug later as well in node_type");
                            map.insert(
                                Sha256Hash::from_string(elements[1]),
                                DockerImage::from(elements[0]),
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

    fn use_repo_tags(image: &ImageSummary, map: &mut HashMap<Sha256Hash, DockerImage>) {
        debug!("REPO DIGESTS BROKEN: {:#?}", image);
        debug!("Using repo_tags instead");
        for tag in &image.repo_tags {
            map.insert(Sha256Hash::from_string(&image.id.clone()), DockerImage::from(tag.as_str()));
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

    async fn list_images(&self) -> HashMap<Sha256Hash, DockerImage> {
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

#[cfg(test)]
mod dockerapi_tests {
    use super::*;
    use std::str::FromStr;

    const TEST_HASH: &str =
        "sha256:5001444e81ee23f9f66beb096b607d2151e1a12acd2733f67cfbb36247e7443b";
    const TEST_BYTES: [u8; 32] = [
        0x50, 0x01, 0x44, 0x4e, 0x81, 0xee, 0x23, 0xf9, 0xf6, 0x6b, 0xeb, 0x09, 0x6b, 0x60, 0x7d,
        0x21, 0x51, 0xe1, 0xa1, 0x2a, 0xcd, 0x27, 0x33, 0xf6, 0x7c, 0xfb, 0xb3, 0x62, 0x47, 0xe7,
        0x44, 0x3b,
    ];

    #[test]
    fn test_parse_and_display() {
        let hash = Sha256Hash::from_string(TEST_HASH);
        assert_eq!(hash.0, TEST_BYTES);
        assert_eq!(hash.to_string(), TEST_HASH);
    }

    #[test]
    fn test_from_bytes() {
        let hash = Sha256Hash::from(TEST_BYTES);
        assert_eq!(hash.to_string(), TEST_HASH);
    }

    #[test]
    fn test_multiple_hashes() {
        let test_cases = [
            "sha256:a33a85525a8a4f95fb3f2bd13c897709e930b376b6770c2ee941d117aff76e7a",
            "sha256:5001444e81ee23f9f66beb096b607d2151e1a12acd2733f67cfbb36247e7443b",
            "sha256:0ddb7a14d16cdc41a73ef2fc4965345661eb4336cf63024a94d7aecc6b36f3c7",
            "sha256:6132897045c12760f19742062670d06810425473ea711786c89d2b4c3a3a31c8",
        ];

        for hash_str in test_cases {
            let hash = Sha256Hash::from_string(hash_str);
            assert_eq!(hash.to_string(), hash_str);
        }
    }

    #[test]
    fn test_equality() {
        let hash1 = Sha256Hash::from_string(TEST_HASH);
        let hash2 = Sha256Hash::from(TEST_BYTES);
        assert_eq!(hash1, hash2);
    }

    #[test]
    #[should_panic]
    fn test_invalid_string_length() {
        let invalid_hash = "sha256:123"; // Too short
        Sha256Hash::from_string(invalid_hash);
    }

    #[test]
    #[should_panic]
    fn test_invalid_prefix() {
        let invalid_hash = "md5:5001444e81ee23f9f66beb096b607d2151e1a12acd2733f67cfbb36247e7443b";
        Sha256Hash::from_string(invalid_hash);
    }
}
