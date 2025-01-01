use std::time::Duration;

use bollard::container::LogOutput;
use ivynet_docker::{container::Container, dockerapi::DockerClient};
use tokio::{task::JoinSet, time};
use tokio_stream::StreamExt;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    grpc::messages::SignedLog,
    signature::sign_string,
    telemetry::{dispatch::TelemetryDispatchHandle, ConfiguredAvs},
    wallet::IvyWallet,
};

type LogListenerResult = Result<ListenerData, LogListenerError>;

/// Manager for a set of LogsListeners. This will spawn and manage the underlying listeners as
/// futures, and is made accessible via the `LogsListenerHandle`.
#[derive(Debug)]
pub struct LogsListenerManager {
    docker: DockerClient,
    signer: IvyWallet,
    machine_id: Uuid,
    dispatcher: TelemetryDispatchHandle,
    listener_set: JoinSet<LogListenerResult>,
}

impl LogsListenerManager {
    pub fn new(
        docker: &DockerClient,
        signer: &IvyWallet,
        machine_id: Uuid,
        dispatcher: &TelemetryDispatchHandle,
    ) -> Self {
        Self {
            docker: docker.clone(),
            signer: signer.clone(),
            machine_id,
            dispatcher: dispatcher.clone(),
            listener_set: JoinSet::new(),
        }
    }

    /// Add a listener to the manager as a future. The listener will be spawned and run in the
    /// background. The future will resolve to the container that the listener is listening to once
    /// the stream is closed for further handling, restarts, etc.
    pub async fn add_listener(
        &mut self,
        container: &Container,
        node_data: &ConfiguredAvs,
    ) -> Result<(), LogListenerError> {
        let listener_data = ListenerData {
            container: container.clone(),
            node_data: node_data.clone(),
            machine_id: self.machine_id,
            identity_wallet: self.signer.clone(),
        };
        self.add_listener_from_data(&listener_data).await
    }

    pub async fn add_listener_from_data(
        &mut self,
        data: &ListenerData,
    ) -> Result<(), LogListenerError> {
        // TODO: Have not rely on ConfiguredAvs
        let listener =
            LogsListener::new(self.docker.clone(), self.dispatcher.clone(), data.clone());
        self.listener_set.spawn(async move { listener_fut(listener).await });
        info!("Added log listener for container: {}", data.node_data.container_name);
        Ok(())
    }
}

// TODO: Not a huge fan of cloning machine_id and identity wallet to this struct via ListenerData
// for singing, as there will be potentially lots of instances of this and it feels like a waste.
// Cleaner pattern may be to have a signing service or actor that can be shared across listeners.

/// An individual instance of a LogListener, which listens to the logs of a single container and
/// sends them to the dispatcher.
struct LogsListener {
    docker: DockerClient,
    dispatcher: TelemetryDispatchHandle,
    listener_data: ListenerData,
}

#[derive(Debug, Clone)]
pub struct ListenerData {
    pub container: Container,
    pub node_data: ConfiguredAvs,
    pub machine_id: Uuid,
    pub identity_wallet: IvyWallet,
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
                    // error!("{}", format!("Log read error | {} | : {}", self.container.image(),
                    // e));
                    return Err(LogListenerError::DockerError(e));
                }
            }
        }
        info!("Log stream closed for container: {}", self.listener_data.node_data.container_name);
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
