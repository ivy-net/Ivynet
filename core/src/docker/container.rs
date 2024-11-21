use bollard::{
    container::{LogOutput, LogsOptions},
    secret::ContainerSummary,
};
use tokio::task::JoinHandle;
use tokio_stream::{Stream, StreamExt};
use tracing::{error, info};

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

pub struct LogsListenerManager(Vec<JoinHandle<Result<(), LogListenerError>>>);

impl LogsListenerManager {
    pub fn new(handles: Vec<JoinHandle<Result<(), LogListenerError>>>) -> Self {
        Self(handles)
    }

    pub async fn add_listener(&mut self, docker: &DockerClient, container: &Container) {
        let mut listener = LogsListener::new(docker.clone(), container.clone());
        let handle = tokio::spawn(async move {
            loop {
                if let Err(e) = listener.try_listen().await {
                    error!("Listener error: {}", e);
                    return Err(e);
                }
                info!(
                    "{}",
                    format!(
                        "Listener for {:?} closed, attempting to reconnect...",
                        listener.container.image()
                    )
                );
                // Wait a sec for the container to come up, also don't spam
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });
        self.0.push(handle);
    }
}

pub struct LogsListener {
    pub docker: DockerClient,
    pub container: Container,
}

impl LogsListener {
    pub fn new(docker: DockerClient, container: Container) -> Self {
        Self { docker, container }
    }

    async fn try_listen(&mut self) -> Result<(), LogListenerError> {
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
        println!("LOGMSG: {:?}", log);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LogListenerError {
    #[error("Docker API error: {0}")]
    DockerError(#[from] bollard::errors::Error),
}
