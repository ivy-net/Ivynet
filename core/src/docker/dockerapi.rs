use bollard::{secret::ContainerSummary, Docker};

use crate::node_type::NodeType;

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
    fn new(docker: Docker) -> Self {
        Self(docker)
    }
}

impl Default for DockerClient {
    fn default() -> Self {
        Self(connect_docker())
    }
}

impl DockerClient {
    pub async fn list_containers(&self) -> Vec<ContainerSummary> {
        self.0.list_containers::<String>(None).await.expect("Cannot list containers")
    }

    /// Inspect a container by image name
    pub async fn inspect(&self, image_name: &str) -> Option<Container> {
        let containers = self.list_containers().await;
        for container in containers {
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

    /// Find an active container for a given node type
    pub async fn find_node_container(&self, node_type: &NodeType) -> Option<Container> {
        let image_name = node_type.default_docker_image_name().unwrap();
        self.inspect(image_name).await
    }

    /// Find all active containers for a slice of node types
    pub async fn find_node_containers(&self, node_types: &[NodeType]) -> Vec<Container> {
        let image_names: Vec<&str> = node_types
            .iter()
            .map(|node_type| node_type.default_docker_image_name().unwrap())
            .collect();
        self.inspect_many(&image_names).await
    }

    /// Find all active containers for all available node types
    pub async fn find_all_node_containers(&self) -> Vec<Container> {
        let node_types = NodeType::all();
        self.find_node_containers(&node_types).await
    }
}

pub struct Container(pub ContainerSummary);

impl Container {
    pub fn new(container: ContainerSummary) -> Self {
        Self(container)
    }

    /// Container ID
    pub fn id(&self) -> Option<&str> {
        self.0.id.as_ref().map(|s| s.as_str())
    }

    /// Image ID for the associated container
    pub fn image_id(&self) -> Option<&str> {
        self.0.image_id.as_ref().map(|s| s.as_str())
    }

    /// Image name for the associated container
    pub fn image(&self) -> Option<&str> {
        self.0.image.as_ref().map(|s| s.as_str())
    }

    pub fn ports(&self) -> Option<&Vec<bollard::models::Port>> {
        self.0.ports.as_ref()
    }

    pub fn public_ports(&self) -> Vec<u16> {
        self.ports()
            .map(|ports| ports.iter().filter_map(|port| port.public_port).collect())
            .unwrap_or_default()
    }
}
