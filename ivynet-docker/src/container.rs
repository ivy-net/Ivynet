use std::str::FromStr;

use super::dockerapi::DockerClient;
use bollard::{
    container::{LogOutput, LogsOptions},
    secret::ContainerSummary,
};
use tokio_stream::Stream;

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

    pub fn public_ports(&self) -> Vec<u16> {
        self.ports()
            .map(|ports| ports.iter().filter_map(|port| port.public_port).collect())
            .unwrap_or_default()
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
}
