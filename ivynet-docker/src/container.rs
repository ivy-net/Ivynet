use std::{str::FromStr, time::Duration};

use crate::{
    dockerapi::DockerApi,
    sidecar::{
        build_sidecar_image,
        netstat::{self, NetstatEntry},
        DockerSidecarError,
    },
};

use super::dockerapi::DockerClient;

use bollard::{
    container::{Config, CreateContainerOptions, LogOutput, LogsOptions},
    errors::Error,
    secret::{ContainerSummary, HostConfig},
};
use futures::TryStreamExt;
use tokio_stream::Stream;
use tracing::info;

/// Type representing a docker image verison `repository:tag.` Primarily for tracking image version
/// between container and image.
pub struct DockerImageVersion {
    pub repository: String,
    pub tag: String,
}

impl FromStr for DockerImageVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            return Err("Invalid image version string".to_string());
        }
        Ok(Self { repository: parts[0].to_string(), tag: parts[1].to_string() })
    }
}

#[derive(Clone, Debug)]
pub struct Container(pub ContainerSummary);

impl Container {
    pub fn new(container: ContainerSummary) -> Self {
        Self(container)
    }

    /// Returns a vec of container names. Container names must be unique per docker daemon.
    pub fn names(&self) -> Option<&Vec<String>> {
        self.0.names.as_ref()
    }

    /// Container ID
    pub fn id(&self) -> Option<&str> {
        self.0.id.as_deref()
    }

    /// Image ID for the associated container
    pub fn image_id(&self) -> Option<&str> {
        self.0.image_id.as_deref()
    }

    /// Image name for the associated container
    pub fn image(&self) -> Option<&str> {
        self.0.image.as_deref()
    }

    pub fn ports(&self) -> Option<&Vec<bollard::models::Port>> {
        self.0.ports.as_ref()
    }

    pub fn state(&self) -> Option<&str> {
        self.0.state.as_deref()
    }

    pub async fn public_ports(&self, docker: &impl DockerApi) -> Vec<u16> {
        match self.is_network_mode_host() {
            true => self.get_host_ports(docker).await.unwrap_or_default(),
            false => self
                .ports()
                .map(|ports| ports.iter().filter_map(|port| port.public_port).collect())
                .unwrap_or_default(),
        }
    }

    pub async fn metrics_port(&self, docker: &DockerClient) -> Option<u16> {
        let mut ports = self.public_ports(docker).await;
        ports.sort();
        ports.dedup();
        for port in ports {
            if (reqwest::Client::new()
                .get(format!("http://localhost:{}/metrics", port))
                .timeout(Duration::from_secs(5))
                .send()
                .await)
                .is_ok()
            {
                return Some(port);
            }
        }
        None
    }

    /// Stream logs for the container since a given timestamp. Returns a stream of log outputs, or
    /// a None if the container
    pub fn stream_logs(
        &self,
        docker: &DockerClient,
        since: i64,
    ) -> impl Stream<Item = Result<LogOutput, bollard::errors::Error>> {
        let log_opts: LogsOptions<&str> =
            LogsOptions { follow: true, stdout: true, stderr: true, since, ..Default::default() };
        docker.0.logs(self.id().unwrap(), Some(log_opts))
    }

    /// Stream logs for the container since current time
    pub fn stream_logs_latest(
        &self,
        docker: &DockerClient,
    ) -> impl Stream<Item = Result<LogOutput, bollard::errors::Error>> {
        let now = chrono::Utc::now().timestamp();
        self.stream_logs(docker, now)
    }

    /// Get the ports exposed by the container on the host machine / network
    pub async fn get_host_ports(
        &self,
        docker: &impl DockerApi,
    ) -> Result<Vec<u16>, ContainerError> {
        let sidecar_name = build_sidecar_image(&docker.inner()).await?;
        let sidecar_container = self.run_one_shot_sidecar(docker, &sidecar_name).await?;

        let mut logs = docker.stream_logs_by_container_id(&sidecar_container, 0).await;

        let mut ports = Vec::new();

        // collect logs to a vec
        while let Some(log) = logs.try_next().await? {
            let entry = NetstatEntry::from_str(&log.to_string())?;
            let port = match entry.local_port.parse::<u16>() {
                Ok(port) => port,
                Err(_) => continue,
            };
            ports.push(port);
        }

        Ok(ports)
    }

    fn is_network_mode_host(&self) -> bool {
        if let Some(host_config) = &self.0.host_config {
            if let Some(network_mode) = &host_config.network_mode {
                return network_mode == "host";
            }
        }
        false
    }

    /// Runs a "one-shot" sidecar container for the target container. The sidecar container is
    /// started and then immediately stopped after running its contents. Returns the container ID
    /// of the sidecar container.
    async fn run_one_shot_sidecar(
        &self,
        docker: &impl DockerApi,
        sidecar_image: &str,
    ) -> Result<String, ContainerError> {
        // HostConfig with pid_mode and network_mode set to use the target container
        let container_id = self.id().ok_or(ContainerError::NoContainerId)?.to_string();
        let sidecar_name = format!("ivynet_sidecar_{}", &container_id);
        let host_config = HostConfig {
            pid_mode: Some(format!("container:{}", container_id)),
            network_mode: Some(format!("container:{}", container_id)),
            auto_remove: Some(true),
            ..Default::default()
        };

        let create_options =
            CreateContainerOptions { name: sidecar_name.clone(), ..Default::default() };

        // By default, the container's CMD is ["/bin/sh"]. We'll override it to remain running:
        // "tail -f /dev/null" or "sleep 999999" is a common trick to keep the container alive.
        let config = Config {
            image: Some(sidecar_image),
            host_config: Some(host_config),
            ..Default::default()
        };

        // Create the container
        let create_response = docker
            .inner()
            .create_container(Some(create_options), config)
            .await
            .map_err(ContainerError::CreateContainerError)?;
        let sidecar_id = create_response.id;

        // Start the container
        docker
            .inner()
            .start_container::<String>(&sidecar_id, None)
            .await
            .map_err(ContainerError::StartContainerError)?;

        info!("Sidecar started: {:#?}", sidecar_name);

        docker
            .inner()
            .wait_container::<String>(&sidecar_id, None)
            .try_for_each(|details| async move {
                println!("Container exit details: {:?}", details);
                Ok(())
            })
            .await?;

        Ok(sidecar_id)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    #[error("Container has no ID")]
    NoContainerId,
    #[error("Could not create container: {0}")]
    CreateContainerError(Error),
    #[error("Could not start container: {0}")]
    StartContainerError(Error),
    #[error("Could not find container with name: {0}")]
    NoContainerFound(String),
    #[error("Container is not in host network mode")]
    NotHostNetworkMode,
    #[error(transparent)]
    DockerSidecarError(#[from] DockerSidecarError),
    #[error(transparent)]
    ContainerError(#[from] Error),
    #[error(transparent)]
    ParseError(#[from] netstat::ParseError),
}
