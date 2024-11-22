use std::time::Duration;

use bollard::{
    container::{LogOutput, LogsOptions},
    secret::ContainerSummary,
};
use tokio::{task::JoinSet, time};
use tokio_stream::{Stream, StreamExt};
use tracing::{error, info};

use crate::telemetry::dispatch::TelemetryDispatcher;

use super::dockerapi::DockerClient;

#[derive(Clone)]
pub struct Container(pub ContainerSummary);

impl Container {
    pub fn new(container: ContainerSummary) -> Self {
        Self(container)
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

pub type LogListenerResult = Result<Container, LogListenerError>;
pub struct LogsListenerManager(JoinSet<LogListenerResult>);

impl LogsListenerManager {
    pub fn new() -> Self {
        let futures = JoinSet::new();
        Self(futures)
    }

    /// Add a listener to the manager as a future. The listener will be spawned and run in the
    /// background. The future will resolve to the container that the listener is listening to once
    /// the stream is closed for further handling, restarts, etc.
    pub async fn add_listener(&mut self, docker: &DockerClient, container: &Container) {
        let listener = LogsListener::new(docker.clone(), container.clone());
        // Spawn the listener future
        self.0.spawn(async move {
            if let Err(e) = listener.try_listen().await {
                error!("Listener error: {}", e);
                return Err(e);
            }
            Ok(listener.container.clone())
        });
    }

    pub async fn run(mut self, docker: &DockerClient) {
        while let Some(future) = self.0.join_next().await {
            match future {
                Ok(result) => {
                    match result {
                        Ok(container) => {
                            info!("Listener exited for container: {:?}", container.image());
                            info!(
                                "Attempting to restart listener for container: {:?}",
                                container.image()
                            );
                            // wait a sec for the container to potentially restart
                            time::sleep(Duration::from_secs(5)).await;
                            self.add_listener(docker, &container).await;
                        }
                        Err(e) => {
                            error!("Listener error: {}", e);
                        }
                    };
                }
                Err(e) => {
                    error!("Unexpected listener error: {}, listener exiting...", e);
                }
            }
        }
    }
}

impl Default for LogsListenerManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LogsListener {
    pub docker: DockerClient,
    pub container: Container,
    pub dispatcher: TelemetryDispatcher,
}

impl LogsListener {
    pub fn new(
        docker: DockerClient,
        container: Container,
        dispatcher: TelemetryDispatcher,
    ) -> Self {
        Self { docker, container, dispatcher }
    }

    async fn try_listen(&self) -> Result<(), LogListenerError> {
        let mut stream = self.container.stream_logs_latest(&self.docker);

        while let Some(log_result) = stream.next().await {
            match log_result {
                Ok(log) => {
                    // Process log message
                    self.handle_log(log).await?;
                }
                Err(e) => {
                    return Err(LogListenerError::DockerError(e));
                }
            }
        }
        Ok(())
    }

    async fn handle_log(&self, log: LogOutput) -> Result<(), LogListenerError> {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LogListenerError {
    #[error("Docker API error: {0}")]
    DockerError(#[from] bollard::errors::Error),
    #[error("LogListener error: {0}")]
    LogListenerError(String),
}

#[cfg(test)]
mod tests {}
