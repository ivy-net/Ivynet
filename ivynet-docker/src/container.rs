use std::{fmt::Display, str::FromStr, time::Duration};

use crate::{
    dockerapi::DockerApi,
    repodigest::RepoDigest,
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
    secret::{ContainerSummary, HostConfig, ImageInspect},
    Docker,
};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use tokio_stream::Stream;
use tracing::info;

/// Type representing a docker image verison `repository:tag.` Primarily for tracking image version
/// between container and image.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContainerImage {
    pub repository: String,
    pub tag: Option<String>,
}

impl From<&str> for ContainerImage {
    fn from(value: &str) -> Self {
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() == 2 {
            Self { repository: parts[0].to_string(), tag: Some(parts[1].to_string()) }
        } else {
            Self { repository: value.to_string(), tag: None }
        }
    }
}

impl Display for ContainerImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(tag) = &self.tag {
            write!(f, "{}:{}", self.repository, tag)
        } else {
            write!(f, "{}", self.repository)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContainerId(pub String);

impl From<&str> for ContainerId {
    fn from(value: &str) -> Self {
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() != 2 || parts[0] != "sha256" {
            panic!("Invalid SHA256 hash format");
        }

        let hash = parts[1];
        if hash.len() != 64 {
            panic!("Invalid hash length: {hash}");
        }

        Self(hash.to_owned())
    }
}

impl Display for ContainerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sha256:{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Container(ContainerSummary);

impl Container {
    pub fn new(container: ContainerSummary) -> Self {
        Self(container)
    }

    /// Returns a vec of container names. Container names must be unique per docker daemon.
    pub fn names(&self) -> Option<Vec<String>> {
        let mut names = self.0.names.clone()?;
        // Strip leading slashes from container name
        for name in names.iter_mut() {
            if name.starts_with('/') {
                // This may be faster with a remove(0) call
                *name = name[1..].to_string();
            }
        }
        Some(names)
    }

    /// Container ID
    pub fn id(&self) -> Option<&str> {
        self.0.id.as_deref()
    }

    /// Image ID for the associated container. This is the image ID, not the digest.
    pub fn image_id(&self) -> Option<&str> {
        self.0.image_id.as_deref()
    }

    /// Image name for the associated container
    pub fn image(&self) -> Option<&str> {
        self.0.image.as_deref()
    }

    pub async fn image_inspect(&self, docker: &Docker) -> Option<ImageInspect> {
        let image_id = self.image_id()?;
        docker.inspect_image(image_id).await.ok()
    }

    pub async fn repo_digest(&self, docker: &Docker) -> Option<String> {
        let image_inspect = self.image_inspect(docker).await?;
        let digests = image_inspect.repo_digests?;
        let digest_str = digests.first()?;
        let repo_digest = RepoDigest::from_str(digest_str).ok()?;
        repo_digest.digest
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

#[cfg(test)]
mod tests {
    use crate::dockerapi::DockerApi;

    use super::*;

    #[tokio::test]
    async fn test_container_name_trim() {
        let docker = DockerClient::default();
        let containers = docker.list_containers().await;
        for container in containers {
            let names = container.names().unwrap();
            if names == vec!["eigenda-native-node"] {
                println!("NAMES: {:?}", container.names().unwrap());
                println!("IMAGE: {:?}", container.image().unwrap());
                println!("ID: {:?}", container.id().unwrap());
                println!("IMAGE_ID: {:?}", container.image_id().unwrap());
                println!(
                    "REPO DIGEST: {:?}",
                    container.repo_digest(&docker.inner()).await.unwrap()
                );
                let inspect = docker.0.inspect_image(container.image().unwrap()).await.unwrap();
                println!("INSPECT: {:#?}", inspect);
            }
        }
    }
}
