use core::fmt;
use registry::ImageRegistry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::EnumIter;
use tokio::time::{sleep, Duration};
use tokio_stream::StreamExt;
use tracing::{error, warn};

use ivynet_node_type::{NodeType, NodeTypeError};

pub mod compose_images;
pub mod container;
pub mod dockerapi;
pub mod dockercmd;
pub mod eventstream;
pub mod logs;
pub mod registry;

#[cfg(test)]
pub mod mocks;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum RegistryType {
    DockerHub,
    OtherDockerHub,
    Github,
    GoogleCloud,
    AWS,
    Chainbase,
    Othentic,
}

impl RegistryType {
    pub fn get_registry_hosts() -> Vec<&'static str> {
        vec![
            "registry-1.docker.io",
            "docker.io",
            "ghcr.io",
            "gcr.io",
            "public.ecr.aws",
            "repository.chainbase.com",
            "othentic",
        ]
    }

    pub fn from_host(host: &str) -> Option<Self> {
        match host {
            "registry-1.docker.io" => Some(Self::DockerHub),
            "docker.io" => Some(Self::OtherDockerHub),
            "ghcr.io" => Some(Self::Github),
            "gcr.io" => Some(Self::GoogleCloud),
            "public.ecr.aws" => Some(Self::AWS),
            "repository.chainbase.com" => Some(Self::Chainbase),
            "othentic" => Some(Self::Othentic),
            _ => None,
        }
    }

    pub fn batch_size(&self) -> usize {
        match self {
            Self::AWS => 5, // AWS has stricter rate limits
            _ => 10,
        }
    }

    pub fn retry_delay(&self) -> Duration {
        match self {
            Self::AWS => Duration::from_secs(5),
            _ => Duration::from_secs(1),
        }
    }

    pub fn max_retries(&self) -> u32 {
        match self {
            Self::AWS => 12,
            _ => 4,
        }
    }
}

impl fmt::Display for RegistryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let registry = match self {
            Self::OtherDockerHub => "docker.io",
            Self::DockerHub => "registry-1.docker.io",
            Self::Github => "ghcr.io",
            Self::GoogleCloud => "gcr.io",
            Self::AWS => "public.ecr.aws",
            Self::Chainbase => "repository.chainbase.com",
            Self::Othentic => "Othentic has no registry",
        };
        write!(f, "{}", registry)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error(transparent)]
    RegistryError(#[from] docker_registry::errors::Error),
    #[error(transparent)]
    NodeTypeError(#[from] NodeTypeError),
    #[error("Registry operation failed after retries: {0}")]
    RetryExhausted(String),
}

pub struct DockerRegistry {
    client: docker_registry::v2::Client,
    image: String,
    registry_type: RegistryType,
}

impl DockerRegistry {
    pub fn new(
        client: docker_registry::v2::Client,
        image: &str,
        registry_type: RegistryType,
    ) -> Self {
        Self { client, image: image.to_owned(), registry_type }
    }

    pub async fn from_host_and_repo(host: &str, repo: &str) -> Result<Self, RegistryError> {
        let registry_type = RegistryType::from_host(host)
            .ok_or_else(|| RegistryError::RetryExhausted("Unknown registry host".to_string()))?;

        let client = docker_registry::v2::Client::configure()
            .registry(host)
            .insecure_registry(false)
            .build()?;

        let login_scope = format!("repository:{}:pull", repo);
        let client = client.authenticate(&[&login_scope]).await?;

        Ok(Self::new(client, repo, registry_type))
    }

    pub async fn from_node_type(entry: &NodeType) -> Result<Self, RegistryError> {
        let registry = entry.registry()?;
        let repo = entry.default_repository()?;
        Self::from_host_and_repo(&registry.to_string(), repo).await
    }

    pub async fn get_tags(&self) -> Result<Vec<String>, RegistryError> {
        // A bit terse, explanation: we collect the results of the get_tags stream into a Vec, then
        // iterate over the results, logging any errors and filtering them out. Finally, we
        // collect the results into a Vec.
        let mut retries = 0;
        let max_retries = self.registry_type.max_retries();
        let mut delay = self.registry_type.retry_delay();

        loop {
            match self.client.get_tags(&self.image, Some(50)).collect::<Vec<_>>().await {
                tags if tags.iter().all(|r| r.is_ok()) => {
                    return Ok(tags.into_iter().filter_map(|r| r.ok()).collect());
                }
                _ if retries >= max_retries => {
                    return Err(RegistryError::RetryExhausted("Failed to fetch tags".to_string()));
                }
                _ => {
                    warn!("Retrying tags fetch after delay of {}s", delay.as_secs());
                    sleep(delay).await;
                    delay *= 5;
                    retries += 1;
                }
            }
        }
    }

    /// Fetches content digest for a particular tag. This is the same digest accessible via `docker
    /// image ls --digests`
    pub async fn get_tag_digest(&self, tag: &str) -> Result<Option<String>, DockerRegistryError> {
        self.client.get_manifestref(&self.image, tag).await.map_err(Into::into)
    }
}

pub struct RegistryEntry {
    // TODO: Need to discuss canonical chain-agnostic names for AVSes
    pub name: String,
    pub registry: String,
    pub image: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DockerRegistryError {
    #[error(transparent)]
    RegistryError(#[from] docker_registry::errors::Error),
    #[error(transparent)]
    NodeTypeError(#[from] NodeTypeError),
}

pub fn get_node_type(
    hashes: &HashMap<String, NodeType>,
    hash: &str,
    image_name: &str,
    container_name: &str,
) -> Option<NodeType> {
    let node_type = hashes
        .get(hash)
        .copied()
        .or_else(|| NodeType::from_image(&extract_image_name(image_name)))
        .or_else(|| NodeType::from_default_container_name(container_name.trim_start_matches('/')));
    if node_type.is_none() {
        warn!("No node type found for {}", image_name);
    }
    node_type
}

fn extract_image_name(image_name: &str) -> String {
    RegistryType::get_registry_hosts()
        .into_iter()
        .find_map(|registry| {
            image_name.contains(registry).then(|| {
                image_name
                    .split(&registry)
                    .last()
                    .unwrap_or(image_name)
                    .trim_start_matches('/')
                    .to_string()
            })
        })
        .unwrap_or_else(|| image_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_image_name() {
        let test_cases = vec![
            // Standard registry cases
            ("docker.io/ubuntu:latest", "ubuntu:latest"),
            ("gcr.io/project/image:v1", "project/image:v1"),
            ("ghcr.io/owner/repo:tag", "owner/repo:tag"),
            ("public.ecr.aws/image:1.0", "image:1.0"),
            // Edge cases
            ("ubuntu:latest", "ubuntu:latest"), // No registry
            ("", ""),                           // Empty string
            ("repository.chainbase.com/", ""),  // Just registry
            // Multiple registry-like strings
            ("gcr.io/docker.io/image", "image"), // Should match first registry
            // With and without tags
            ("docker.io/image", "image"),
            ("docker.io/org/image:latest", "org/image:latest"),
            // Special characters
            ("docker.io/org/image@sha256:123", "org/image@sha256:123"),
            ("docker.io/org/image_name", "org/image_name"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                extract_image_name(input),
                expected.to_string(),
                "Failed on input: {}",
                input
            );
        }
    }
}

#[cfg(test)]
mod docker_registry_tests {

    use super::*;

    #[test]
    fn test_registry_from_host() {
        assert_eq!(RegistryType::from_host("ghcr.io"), Some(RegistryType::Github));
        assert_eq!(RegistryType::from_host("invalid"), None);
    }

    #[test]
    fn test_registry_host() {
        assert_eq!(RegistryType::Github.to_string(), "ghcr.io");
    }

    #[tokio::test]
    async fn test_tags() -> Result<(), Box<dyn std::error::Error>> {
        let registry = "ghcr.io";
        let image = "layr-labs/eigenda/opr-node";
        println!("[{}] requesting tags for image {}", registry, image);

        let client: DockerRegistry = DockerRegistry::from_host_and_repo(registry, image).await?;
        let tags = client.get_tags().await?;
        assert!(!tags.is_empty());

        let digest = client.get_tag_digest("0.8.4").await;
        println!("digest: {:?}", digest);

        Ok(())
    }

    #[tokio::test]
    async fn test_registry_entry_tags() -> Result<(), Box<dyn std::error::Error>> {
        let all_entries = NodeType::all_known();
        for entry in all_entries {
            let registry = entry.registry()?;
            let repo = entry.default_repository()?;
            println!("[{}] requesting tags for image {}", registry, repo);
            let client: DockerRegistry =
                DockerRegistry::from_host_and_repo(&registry.to_string(), repo).await?;
            let tags = client.get_tags().await?;
            println!("Assert tags for image {}", registry);
            assert!(!tags.is_empty());
            let _digest = client.get_tag_digest(&tags[0]).await?;
        }
        Ok(())
    }
    #[tokio::test]
    async fn test_get_eigenda_digests() -> Result<(), Box<dyn std::error::Error>> {
        let node_type = NodeType::EigenDA;

        let client = DockerRegistry::from_node_type(&node_type).await?;
        let tags = client.get_tags().await?;
        assert!(!tags.is_empty());
        let mut digests = Vec::new();

        for tag in tags.iter() {
            let digest = client.get_tag_digest(tag).await?;
            if let Some(digest) = digest {
                digests.push(digest);
            }
        }
        assert_eq!(tags.len(), digests.len());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_lagrage_zk_worker_holesky_digest() -> Result<(), Box<dyn std::error::Error>> {
        let node_type = NodeType::LagrangeZkWorkerHolesky;

        let client = DockerRegistry::from_node_type(&node_type).await?;
        let tags = client.get_tags().await?;
        assert!(!tags.is_empty());
        let mut digests = Vec::new();

        for tag in tags.iter() {
            let digest = client.get_tag_digest(tag).await?;
            if let Some(digest) = digest {
                digests.push(digest);
            }
        }
        assert_eq!(tags.len(), digests.len());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_hyperlane_digests() -> Result<(), Box<dyn std::error::Error>> {
        let node_type = NodeType::Hyperlane;
        let client = DockerRegistry::from_node_type(&node_type).await?;
        let tags = client.get_tags().await?;
        assert!(!tags.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_avaprotocol_digests() -> Result<(), Box<dyn std::error::Error>> {
        let node_type = NodeType::AvaProtocol;
        let client = DockerRegistry::from_node_type(&node_type).await?;
        let tags = client.get_tags().await?;
        assert!(!tags.is_empty());
        let mut digests = Vec::new();

        for tag in tags.iter() {
            let digest = client.get_tag_digest(tag).await?;
            if let Some(digest) = digest {
                digests.push(digest);
            }
        }
        assert_eq!(tags.len(), digests.len());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_k3_digests() -> Result<(), Box<dyn std::error::Error>> {
        let node_type = NodeType::K3LabsAvs;
        let client = DockerRegistry::from_node_type(&node_type).await?;
        let tags = client.get_tags().await?;
        assert!(!tags.is_empty());
        let mut digests = Vec::new();

        for tag in tags.iter() {
            let digest = client.get_tag_digest(tag).await?;
            if let Some(digest) = digest {
                digests.push(digest);
            }
        }
        assert_eq!(tags.len(), digests.len());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_eoracle_digests() -> Result<(), Box<dyn std::error::Error>> {
        let node_type = NodeType::EOracle;
        let client = DockerRegistry::from_node_type(&node_type).await?;
        let tags = client.get_tags().await?;
        assert!(!tags.is_empty());
        let mut digests = Vec::new();

        for tag in tags.iter() {
            let digest = client.get_tag_digest(tag).await?;
            if let Some(digest) = digest {
                digests.push(digest);
            }
        }
        assert_eq!(tags.len(), digests.len());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_witnesschain_digests() -> Result<(), Box<dyn std::error::Error>> {
        let node_type = NodeType::WitnessChain;
        let client = DockerRegistry::from_node_type(&node_type).await?;
        let tags = client.get_tags().await?;
        assert!(!tags.is_empty());
        let mut digests = Vec::new();

        for tag in tags.iter() {
            let digest = client.get_tag_digest(tag).await?;
            if let Some(digest) = digest {
                digests.push(digest);
            }
        }
        assert_eq!(tags.len(), digests.len());
        Ok(())
    }
}
