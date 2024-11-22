use std::time::Duration;

use bollard::{
    container::{LogOutput, LogsOptions},
    secret::ContainerSummary,
};
use tokio::{task::JoinSet, time};
use tokio_stream::{Stream, StreamExt};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    grpc::messages::SignedLog,
    signature::sign_string,
    telemetry::{dispatch::TelemetryDispatchHandle, ConfiguredAvs},
    wallet::IvyWallet,
};

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

pub type LogListenerResult = Result<ListenerData, LogListenerError>;
pub struct LogsListenerManager {
    listener_set: JoinSet<LogListenerResult>,
    dispatcher: TelemetryDispatchHandle,
    docker: DockerClient,
}
impl LogsListenerManager {
    pub fn new(dispatcher: TelemetryDispatchHandle, docker: DockerClient) -> Self {
        Self { listener_set: JoinSet::new(), dispatcher, docker }
    }

    /// Add a listener to the manager as a future. The listener will be spawned and run in the
    /// background. The future will resolve to the container that the listener is listening to once
    /// the stream is closed for further handling, restarts, etc.
    pub async fn add_listener(&mut self, data: ListenerData) {
        let listener = LogsListener::new(self.docker.clone(), self.dispatcher.clone(), data);
        // Spawn the listener future
        self.listener_set.spawn(async move { listener_fut(listener).await });
    }

    pub async fn run(mut self) {
        while let Some(future) = self.listener_set.join_next().await {
            match future {
                Ok(result) => {
                    match result {
                        Ok(data) => {
                            info!("Listener exited for container: {:?}", data.container.image());
                            info!(
                                "Attempting to restart listener for container: {:?}",
                                data.container.image()
                            );
                            // wait a sec for the container to potentially restart
                            time::sleep(Duration::from_secs(5)).await;
                            self.add_listener(data).await;
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

// TODO: Not a huge fan of cloning machine_id and identity wallet to this struct via ListenerData
// for singing, as there will be potentially lots of instances of this and it feels like a waste.
// Cleaner pattern may be to have a signing service or actor that can be shared across listeners.
struct LogsListener {
    docker: DockerClient,
    dispatcher: TelemetryDispatchHandle,
    listener_data: ListenerData,
}

struct ListenerData {
    container: Container,
    node_data: ConfiguredAvs,
    machine_id: Uuid,
    identity_wallet: IvyWallet,
}

impl LogsListener {
    pub fn new(
        docker: DockerClient,
        dispatcher: TelemetryDispatchHandle,
        listener_data: ListenerData,
    ) -> Self {
        Self { docker, dispatcher, listener_data }
    }

    async fn try_listen(&self) -> Result<(), LogListenerError> {
        let mut stream = self.listener_data.container.stream_logs_latest(&self.docker);

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
        let log = log.to_string();
        let signature = sign_string(&log, &self.listener_data.identity_wallet)?.to_vec();
        let signed = SignedLog {
            signature,
            machine_id: self.listener_data.machine_id.into(),
            avs_name: self.listener_data.node_data.name.clone(),
            log,
        };
        self.dispatcher.send_log(signed).await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LogListenerError {
    #[error("Docker API error: {0}")]
    DockerError(#[from] bollard::errors::Error),
    #[error("LogListener error: {0}")]
    LogListenerError(String),
    #[error("Signature error: {0}")]
    SignatureError(#[from] crate::signature::IvySigningError),
    #[error("Telemetry dispatch error: {0}")]
    TelemetryDispatchError(#[from] crate::telemetry::dispatch::TelemetryDispatchError),
}

/// Listener future for processing the stream. Yields the data for the container that the listener
/// was listening to once the stream is closed.
pub async fn listener_fut(listener: LogsListener) -> Result<ListenerData, LogListenerError> {
    if let Err(e) = listener.try_listen().await {
        error!("Listener error: {}", e);
        return Err(e);
    }
    Ok(listener.listener_data)
}

#[cfg(test)]
mod tests {}
