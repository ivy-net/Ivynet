use tokio_stream::StreamExt;

use crate::node_type::{NodeType, NodeTypeError};

pub mod compose_images;
pub mod container;
pub mod dockerapi;
pub mod dockercmd;
pub mod logs;

pub struct DockerRegistry {
    client: docker_registry::v2::Client,
    image: String,
}

impl DockerRegistry {
    pub fn new(client: docker_registry::v2::Client, image: &str) -> Self {
        Self { client, image: image.to_owned() }
    }

    pub async fn from_host_and_repo(host: &str, repo: &str) -> Result<Self, DockerRegistryError> {
        let client = docker_registry::v2::Client::configure()
            .registry(host)
            .insecure_registry(false)
            .build()?;
        let login_scope = format!("repository:{}:pull", repo);
        let client = client.authenticate(&[&login_scope]).await?;
        Ok(Self::new(client, repo))
    }

    pub async fn from_node_type(entry: &NodeType) -> Result<Self, DockerRegistryError> {
        let registry = entry.registry()?;
        let repo = entry.default_repository()?;
        Self::from_host_and_repo(registry, repo).await
    }

    pub async fn get_tags(&self) -> Result<Vec<String>, DockerRegistryError> {
        // A bit terse, explanation: we collect the results of the get_tags stream into a Vec, then
        // iterate over the results, logging any errors and filtering them out. Finally, we
        // collect the results into a Vec.
        let res = self
            .client
            .get_tags(&self.image, Some(20))
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|r| r.map_err(|e| tracing::error!("Error fetching tags: {}", e)))
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        Ok(res)
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

#[cfg(test)]
mod docker_registry_tests {

    use super::*;

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
            let client: DockerRegistry = DockerRegistry::from_host_and_repo(registry, repo).await?;
            let tags = client.get_tags().await?;
            println!("Tags for image {}: {:?}", registry, tags.to_vec());
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
            let digest = client.get_tag_digest(&tag).await?;
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
            let digest = client.get_tag_digest(&tag).await?;
            if let Some(digest) = digest {
                digests.push(digest);
            }
        }
        assert_eq!(tags.len(), digests.len());
        Ok(())
    }

    // #[tokio::test]
    // async fn test_get_lagrage_state_committee_digests() -> Result<(), Box<dyn std::error::Error>>
    // {     let node_type = NodeType::LagrangeStateCommittee;

    //     let client = DockerRegistry::from_node_type(&node_type).await?;
    //     let tags = client.get_tags().await?;
    //     assert!(!tags.is_empty());
    //     let mut digests = Vec::new();

    //     for tag in tags.iter() {
    //         let digest = client.get_tag_digest(tag).await?;
    //         if let Some(digest) = digest {
    //             digests.push(digest);
    //         }
    //     }
    //     assert_eq!(tags.len(), digests.len());
    //     Ok(())
    // }

    #[tokio::test]
    #[ignore]
    async fn test_get_hyperlane_digests() -> Result<(), Box<dyn std::error::Error>> {
        let node_type = NodeType::Hyperlane;
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
    async fn get_eoracle_digests() -> Result<(), Box<dyn std::error::Error>> {
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
}
