use bollard::{secret::ContainerSummary, Docker};
use tracing::{debug, info};

pub async fn connect_docker() -> Docker {
    std::env::var("DOCKER_HOST").map(|_| Docker::connect_with_defaults().unwrap()).unwrap_or_else(
        |_| {
            Docker::connect_with_local_defaults()
                .expect("Cannot connect to docker sock. Please set $DOCKER_HOST")
        },
    )
}

pub async fn inspect(image_name: &str) -> Option<ContainerSummary> {
    let docker = connect_docker().await;
    info!("Docker connect result: {:#?}", docker);

    let containers = docker.list_containers::<String>(None).await.ok();

    if let Some(container) = containers {
        for container in container {
            if let Some(image_string) = container.clone().image {
                debug!("Image being searched: {:#?}", image_name);
                debug!("Image found: {:#?}", image_string);
                if image_string.contains(image_name) {
                    debug!("AVS FOUND -> {:#?}", container);
                    return Some(container);
                }
            }
        }
    }
    None
}

pub fn get_active_ports(container: &ContainerSummary) -> Vec<u16> {
    container
        .ports
        .as_ref()
        .map(|ports| ports.iter().filter_map(|port| port.public_port).collect())
        .unwrap_or_default()
}
