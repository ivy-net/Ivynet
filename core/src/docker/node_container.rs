use bollard::{secret::ContainerSummary, Docker};

pub async fn containers() -> Result<Vec<ContainerSummary>, NodeContainerError> {
    let docker = Docker::connect_with_local_defaults()?;
    Ok(docker.list_containers::<String>(None).await.unwrap())
}

pub trait NodeContainer {
    fn get_active_ports(&self) -> Vec<u16>;
    fn has_metrics(&self) -> bool;
}

impl NodeContainer for ContainerSummary {
    fn get_active_ports(&self) -> Vec<u16> {
        self.ports
            .as_ref()
            .map(|ports| ports.iter().filter_map(|port| port.public_port).collect())
            .unwrap_or_default()
    }

    fn has_metrics(&self) -> bool {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeContainerError {
    #[error(transparent)]
    BollardError(#[from] bollard::errors::Error),
}
