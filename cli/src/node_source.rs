use futures::stream::{iter, StreamExt};
use ivynet_docker::{container::Container, dockerapi::DockerApi};
use ivynet_grpc::async_trait;

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

        let mut potentials = Vec::new();

        for container in containers {
            let (names, image_name) = match (container.names(), container.image()) {
                (Some(n), Some(i)) => (n, i),
                _ => continue,
            };

            let mut ports = container.public_ports(self).await;
            ports.sort_unstable();
            ports.dedup();

            if let Some(image_hash) = images.get(image_name) {
                potentials.push(PotentialAvs {
                    container_name: names.first().unwrap_or(&image_name.to_string()).to_string(),
                    image_name: image_name.to_string(),
                    image_hash: image_hash.clone(),
                    ports,
                });
            } else if let Some(key) = images.keys().find(|key| key.contains(image_name)) {
                let image_hash = images.get(key).unwrap();
                potentials.push(PotentialAvs {
                    container_name: names.first().unwrap_or(key).to_string(),
                    image_name: key.clone(),
                    image_hash: image_hash.clone(),
                    ports,
                });
            }
        }

        potentials
    }
}
