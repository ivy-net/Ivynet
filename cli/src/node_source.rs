use ivynet_docker::dockerapi::{DockerApi, Sha256Hash};
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
            let (names, image_str, image_id) =
                match (container.names(), container.image(), container.image_id()) {
                    (Some(n), Some(i), Some(id)) => (n, i, id),
                    _ => continue,
                };

            let mut ports = container.public_ports(self).await;
            ports.sort_unstable();
            ports.dedup();

            let image_hash = Sha256Hash::from_string(image_id);

            potentials.push(PotentialAvs {
                container_name: names.first().unwrap_or(&image_str.to_string()).to_string(),
                docker_image: image_str.into(),
                manifest: image_hash.clone(),
                ports,
            });

            // if let Some(docker_image) = images.get(&image_hash) {}
            //Shouldn't be needed with sha256 hash as key
            //  else if let Some(key) = images.keys().find(|key| key.contains(image_name)) {
            //     let image_hash = images.get(key).unwrap();
            //     potentials.push(PotentialAvs {
            //         container_name: names.first().unwrap_or(key).to_string(),
            //         image_name: key.clone(),
            //         image_hash: image_hash.clone(),
            //         ports,
            //     });
            // }
        }

        potentials
    }
}
