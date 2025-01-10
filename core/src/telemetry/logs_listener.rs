use std::time::Duration;

use bollard::container::LogOutput;
use ivynet_docker::{container::Container, dockerapi::DockerClient};
use ivynet_grpc::messages::SignedLog;
use ivynet_signer::IvyWallet;
use tokio::{task::JoinSet, time};
use tokio_stream::StreamExt;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    signature::sign_string,
    telemetry::{dispatch::TelemetryDispatchHandle, ConfiguredAvs},
};

type LogListenerResult = Result<ListenerData, LogListenerError>;
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

    pub async fn listen(mut self) -> Result<(), LogListenerError> {
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
                            return Err(e);
                        }
                    };
                }
                Err(e) => {
                    error!("Unexpected listener error: {}, listener exiting...", e);
                    return Err(e.into());
                }
            }
        }
        Ok(())
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

pub struct ListenerData {
    container: Container,
    node_data: ConfiguredAvs,
    machine_id: Uuid,
    identity_wallet: IvyWallet,
}

impl ListenerData {
    pub fn new(
        container: Container,
        node_data: ConfiguredAvs,
        machine_id: Uuid,
        identity_wallet: IvyWallet,
    ) -> Self {
        Self { container, node_data, machine_id, identity_wallet }
    }
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
        time::sleep(Duration::from_secs(10)).await;
        let mut stream = self.listener_data.container.stream_logs_latest(&self.docker);

        while let Some(log_result) = stream.next().await {
            match log_result {
                Ok(log) => {
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
        // println!("log: {:#?}", log);
        let log = log.to_string();
        let signature = sign_string(&log, &self.listener_data.identity_wallet)?.to_vec();
        let signed = SignedLog {
            signature,
            machine_id: self.listener_data.machine_id.into(),
            avs_name: self.listener_data.node_data.assigned_name.clone(),
            log: log.clone(),
        };
        match self.dispatcher.send_log(signed).await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to send or save log: {} | With log: {}", e, &log);
            }
        };
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
    #[error("Unexpected error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

/// Listener future for processing the stream. Yields the data for the container that the listener
/// was listening to once the stream is closed.
async fn listener_fut(listener: LogsListener) -> Result<ListenerData, LogListenerError> {
    if let Err(e) = listener.try_listen().await {
        error!("Listener error: {}", e);
        return Err(e);
    }
    Ok(listener.listener_data)
}
