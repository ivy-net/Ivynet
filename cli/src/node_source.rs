use ivynet_docker::{container::ContainerId, dockerapi::DockerApi};
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
        let containers = self.list_containers().await;

        let mut potentials = Vec::new();

        for container in containers {
            let (names, image_str, image_id) =
                match (container.names(), container.repo_tag(), container.image_id()) {
                    (n, Some(i), Some(id)) => (n, i, id),
                    _ => continue,
                };

            let mut ports = container.public_ports(self).await;
            ports.sort_unstable();
            ports.dedup();

            let image_hash = ContainerId::from(image_id);

            potentials.push(PotentialAvs {
                container_name: names.first().unwrap_or(&image_str.to_string()).to_string(),
                docker_image: image_str.into(),
                manifest: image_hash,
                ports,
            });
        }

        potentials
    }
}
