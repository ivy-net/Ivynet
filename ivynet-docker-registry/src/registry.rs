use ivynet_node_type::{NodeType, NodeTypeError};
use tokio::time::sleep;
use tokio_stream::StreamExt;
use tracing::{error, warn};

use crate::registry_type::RegistryType::{
    self, Chainbase, DockerHub, Github, GoogleCloud, Local, OptInOnly, Othentic, AWS,
};

pub trait ImageRegistry {
    fn registry(&self) -> Result<RegistryType, NodeTypeError>;
}

impl ImageRegistry for NodeType {
    fn registry(&self) -> Result<RegistryType, NodeTypeError> {
        let res = match self {
            Self::BlessB7s => Github,
            Self::Tanssi => DockerHub,
            Self::Redstone => Othentic,
            Self::Bolt(_) => Github,
            Self::Zellular => DockerHub,
            Self::AtlasNetwork => DockerHub,
            Self::Primus => DockerHub,
            Self::Gasp => DockerHub,
            Self::DittoNetwork(_) => DockerHub,
            Self::EigenDA => Github,
            Self::EOracle => DockerHub,
            Self::AvaProtocol => DockerHub,
            Self::LagrangeStateCommittee => DockerHub,
            Self::LagrangeZkWorker => DockerHub,
            Self::LagrangeZKProver => DockerHub,
            Self::K3LabsAvs => DockerHub,
            Self::K3LabsAvsHolesky => DockerHub,
            Self::Predicate => Github,
            Self::Hyperlane(_) => GoogleCloud,
            Self::WitnessChain => DockerHub,
            Self::Altlayer(_altlayer_type) => AWS,
            Self::AltlayerMach(_altlayer_mach_type) => AWS,
            Self::Omni => DockerHub,
            Self::Automata => Github,
            Self::OpenLayerMainnet => GoogleCloud,
            Self::OpenLayerHolesky => GoogleCloud,
            Self::AethosHolesky => Github,
            Self::ArpaNetworkNodeClient => Github,
            Self::ChainbaseNetworkV1 => Chainbase,
            Self::ChainbaseNetwork => Chainbase,
            Self::UngateInfiniRoute(_) => Othentic,
            Self::GoPlusAVS => Local,
            Self::SkateChain(_) => Othentic,
            Self::MishtiNetwork(_) => Othentic,
            Self::Brevis => Local,
            Self::Nuffle => Local,
            Self::AlignedLayer => Local,
            Self::PrimevMevCommit(_) => Local,
            Self::PrimevBidder => Local,
            Self::Blockless => Local,
            Self::Cycle => Local,
            Self::Kalypso => Local,
            Self::RouterXtendNetwork => OptInOnly,
            Self::CapxCloud => OptInOnly,
            Self::Symbiosis => OptInOnly,
            Self::Radius => OptInOnly,
            Self::IBTCNetwork => OptInOnly,
            Self::ZKLink => OptInOnly,
            Self::HyveDA => OptInOnly,
            Self::UnifiAVS => OptInOnly,
            Self::Unknown => return Err(NodeTypeError::InvalidNodeType),
        };
        Ok(res)
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
        let registry_type = RegistryType::from_host(host).ok_or_else(|| {
            RegistryError::RetryExhausted(format!(
                "Unknown registry host '{}' repo '{}'",
                host, repo
            ))
        })?;

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

    pub async fn get_manfiest(
        &self,
        tag: &str,
    ) -> Result<docker_registry::v2::manifest::Manifest, DockerRegistryError> {
        self.client.get_manifest(&self.image, tag).await.map_err(Into::into)
    }

    pub async fn get_blob(&self, digest: &str) -> Result<Vec<u8>, DockerRegistryError> {
        self.client.get_blob(&self.image, digest).await.map_err(Into::into)
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

    use ivynet_node_type::ActiveSet;

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

    // This test is inconsistent due to reliance on http call / remote repository
    #[tokio::test]
    #[ignore]
    async fn test_registry_entry_tags() -> Result<(), Box<dyn std::error::Error>> {
        let all_entries = NodeType::all_known_with_repo();
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
        let node_type = NodeType::LagrangeZkWorker;

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
    async fn test_get_hyperlane_digests() -> Result<(), Box<dyn std::error::Error>> {
        let node_type = NodeType::Hyperlane(ActiveSet::Unknown);
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
