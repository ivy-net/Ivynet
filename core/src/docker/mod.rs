use tokio_stream::StreamExt;

use crate::node_type::NodeType;

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

    pub async fn from_host_and_image(host: &str, image: &str) -> Result<Self, DockerRegistryError> {
        let client = docker_registry::v2::Client::configure()
            .registry(host)
            .insecure_registry(false)
            .build()?;
        let login_scope = format!("repository:{}:pull", image);
        let client = client.authenticate(&[&login_scope]).await?;
        Ok(Self::new(client, image))
    }

    pub async fn from_registry_entry(entry: &RegistryEntry) -> Result<Self, DockerRegistryError> {
        Self::from_host_and_image(&entry.registry, &entry.image).await
    }

    pub async fn from_node_registry_entry(
        entry: &NodeRegistryEntry,
    ) -> Result<Self, DockerRegistryError> {
        let entry = entry.registry_entry();
        Self::from_registry_entry(&entry).await
    }

    pub async fn get_tags(&self) -> Result<Vec<String>, DockerRegistryError> {
        // A bit terse, explanation: we collect the results of the get_tags stream into a Vec, then
        // iterate over the results, logging any errors and filtering them out. Finally, we
        // collect the results into a Vec.
        let res = self
            .client
            .get_tags(&self.image, None)
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

// Associated functions can probably later be implemented for NodeType instead
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum NodeRegistryEntry {
    EigenDA,
    LagrangeZKCoprocessor,
    LagrangeStateCommittee,
    Ava,
    Eoracle,
    K3Labs,
    Hyperlane,
    // Brevis // Probably not possible - github accessible
    // WitnesschainWatchtower,
    // Predicate,
}

impl TryFrom<&str> for NodeRegistryEntry {
    type Error = DockerRegistryError;

    fn try_from(node_type: &str) -> Result<Self, Self::Error> {
        let res = match node_type {
            "eigenda" => NodeRegistryEntry::EigenDA,
            "lagrange-zk-worker" => NodeRegistryEntry::LagrangeZKCoprocessor,
            "lagrange-state-committee" => NodeRegistryEntry::LagrangeStateCommittee,
            "ap_avs" => NodeRegistryEntry::Ava,
            "eoracle" => NodeRegistryEntry::Eoracle,
            "k3-labs-avs-operator" => NodeRegistryEntry::K3Labs,
            "hyperlane-operator" => NodeRegistryEntry::Hyperlane,
            _ => return Err(DockerRegistryError::NodeTypeError(NodeType::from(node_type))),
        };
        Ok(res)
    }
}

// TODO: Eventually deprecate NodeRegistryEntry and run these directly on NodeType
impl TryFrom<&NodeType> for NodeRegistryEntry {
    type Error = DockerRegistryError;

    fn try_from(node_type: &NodeType) -> Result<Self, Self::Error> {
        let res = match node_type {
            NodeType::EigenDA => NodeRegistryEntry::EigenDA,
            NodeType::LagrangeHoleskyWorker => NodeRegistryEntry::LagrangeZKCoprocessor,
            // NodeType::LagrangeStateCommittee => NodeRegistryEntry::LagrangeStateCommittee,
            // NodeType::Ava => NodeRegistryEntry::Ava,
            // NodeType::Eoracle => NodeRegistryEntry::Eoracle,
            // NodeType::K3Labs => NodeRegistryEntry::K3Labs,
            // NodeType::Hyperlane => NodeRegistryEntry::Hyperlane,
            _ => return Err(DockerRegistryError::NodeTypeError(*node_type)),
        };
        Ok(res)
    }
}

impl NodeRegistryEntry {
    pub fn all() -> Vec<NodeRegistryEntry> {
        vec![
            NodeRegistryEntry::EigenDA,
            NodeRegistryEntry::LagrangeZKCoprocessor,
            NodeRegistryEntry::LagrangeStateCommittee,
            NodeRegistryEntry::Ava,
            NodeRegistryEntry::K3Labs,
            // NodeRegistryEntry::Hyperlane,
            // NodeRegistryEntry::Eoracle,
        ]
    }

    pub fn registry_entry(&self) -> RegistryEntry {
        match self {
            NodeRegistryEntry::EigenDA => RegistryEntry {
                name: "eigenda".to_string(),
                registry: "ghcr.io".to_string(),
                image: "layr-labs/eigenda/opr-node".to_string(),
            },
            NodeRegistryEntry::LagrangeZKCoprocessor => RegistryEntry {
                name: "lagrange-zk-worker".to_string(),
                registry: "registry-1.docker.io".to_string(),
                image: "lagrangelabs/worker".to_string(),
            },
            NodeRegistryEntry::LagrangeStateCommittee => RegistryEntry {
                name: "lagrange-state-committee".to_string(),
                registry: "registry-1.docker.io".to_string(),
                image: "lagrangelabs/lagrange-node".to_string(),
            },
            NodeRegistryEntry::Hyperlane => RegistryEntry {
                name: "hyperlane-operator".to_string(),
                registry: "ghcr.io".to_string(),
                image: "".to_string(),
            },
            NodeRegistryEntry::Ava => RegistryEntry {
                name: "ap_avs".to_string(),
                registry: "registry-1.docker.io".to_string(),
                image: "avaprotocol/ap-avs".to_string(),
            },
            // NodeRegistryEntry::WitnesschainWatchtower => {
            //     RegistryEntry { registry: "".to_string(), image: "".to_string() }
            // }
            NodeRegistryEntry::K3Labs => RegistryEntry {
                name: "k3-labs-avs-operator".to_string(),
                registry: "registry-1.docker.io".to_string(),
                image: "k3official/k3-labs-avs-operator".to_string(),
            },
            NodeRegistryEntry::Eoracle => RegistryEntry {
                name: "eoracle".to_string(),
                registry: "ghcr.io".to_string(),
                image: "eoracle-data-validator".to_string(),
            },
        }
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
    #[error("Error fetching node registry entry from node type: {0}")]
    NodeTypeError(NodeType),
}

#[cfg(test)]
mod docker_registry_tests {

    use super::*;

    #[tokio::test]
    async fn test_tags() -> Result<(), Box<dyn std::error::Error>> {
        let registry = "ghcr.io";
        let image = "layr-labs/eigenda/opr-node";
        println!("[{}] requesting tags for image {}", registry, image);

        let client: DockerRegistry = DockerRegistry::from_host_and_image(registry, image).await?;
        let tags = client.get_tags().await?;
        assert!(!tags.is_empty());

        let digest = client.get_tag_digest("0.8.4").await;
        println!("digest: {:?}", digest);

        Ok(())
    }

    #[tokio::test]
    async fn test_registry_entry_tags() -> Result<(), Box<dyn std::error::Error>> {
        let all_entries = NodeRegistryEntry::all();
        for entry in all_entries {
            let entry = entry.registry_entry();
            println!("[{}] requesting tags for image {}", entry.registry, entry.image);
            let client: DockerRegistry =
                DockerRegistry::from_host_and_image(&entry.registry, &entry.image).await?;
            let tags = client.get_tags().await?;
            println!("Tags for image {}: {:?}", entry.image, tags.to_vec());
            assert!(!tags.is_empty());
            let digest = client.get_tag_digest(&tags[0]).await;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_get_eigenda_digests() -> Result<(), Box<dyn std::error::Error>> {
        let registry_entry = NodeRegistryEntry::EigenDA;

        let client: DockerRegistry =
            DockerRegistry::from_node_registry_entry(&registry_entry).await?;
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
    async fn test_get_lagrage_zkcoprocessor_digests() -> Result<(), Box<dyn std::error::Error>> {
        let registry_entry = NodeRegistryEntry::LagrangeZKCoprocessor;

        let client: DockerRegistry =
            DockerRegistry::from_node_registry_entry(&registry_entry).await?;
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
    async fn test_get_lagrage_state_committee_digests() -> Result<(), Box<dyn std::error::Error>> {
        let registry_entry = NodeRegistryEntry::LagrangeStateCommittee;

        let client: DockerRegistry =
            DockerRegistry::from_node_registry_entry(&registry_entry).await?;
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
    #[ignore]
    async fn test_get_hyperlane_digests() -> Result<(), Box<dyn std::error::Error>> {
        let registry_entry = NodeRegistryEntry::Hyperlane;

        let client: DockerRegistry =
            DockerRegistry::from_node_registry_entry(&registry_entry).await?;
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
        let registry_entry = NodeRegistryEntry::Ava;

        let client: DockerRegistry =
            DockerRegistry::from_node_registry_entry(&registry_entry).await?;
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
        let registry_entry = NodeRegistryEntry::K3Labs;

        let client: DockerRegistry =
            DockerRegistry::from_node_registry_entry(&registry_entry).await?;
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
    async fn get_get_eoracle_digests() -> Result<(), Box<dyn std::error::Error>> {
        let registry_entry = NodeRegistryEntry::Eoracle;

        let client: DockerRegistry =
            DockerRegistry::from_node_registry_entry(&registry_entry).await?;
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
