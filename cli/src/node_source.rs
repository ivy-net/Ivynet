use ivynet_docker::dockerapi::DockerApi;
use ivynet_grpc::async_trait;
use tracing::debug;

use crate::monitor::PotentialAvs;

#[async_trait]
pub trait NodeSource {
    /// Get a list of potential nodes that can be used with the Ivynet client --
    async fn potential_nodes(&self) -> Vec<PotentialAvs>;
}

#[async_trait]
impl<T: DockerApi> NodeSource for T {
    async fn potential_nodes(&self) -> Vec<PotentialAvs> {
        let images = self.list_images().await;
        let containers = self.list_containers().await;
        containers
            .into_iter()
            .filter_map(|c| {
                let (names, image_name) = (c.names()?, c.image()?);
                let mut ports = match c.ports() {
                    Some(ports) => ports.iter().filter_map(|p| p.public_port).collect::<Vec<_>>(),
                    None => Vec::new(),
                };
                ports.sort();
                ports.dedup();

                if let Some(image_hash) = images.get(image_name) {
                    return Some(PotentialAvs {
                        container_name: names.first().map_or(image_name, |v| v).to_string(),
                        image_name: image_name.to_owned(),
                        image_hash: image_hash.to_string(),
                        ports,
                    });
                } else if let Some(key) = images.keys().find(|key| key.contains(image_name)) {
                    debug!("SHOULD BE: No version tag image: {}", image_name);
                    let image_hash = images.get(key).unwrap();
                    debug!("key (should be with version tag, and its what we'll use for potential avs): {}", key);
                    return Some(PotentialAvs {
                        container_name: names.first().map_or(image_name, |v| v).to_string(),
                        image_name: key.clone(),
                        image_hash: image_hash.to_string(),
                        ports,
                    });
                }
                None
            })
            .collect::<Vec<_>>()
    }
}
