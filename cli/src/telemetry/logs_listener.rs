use std::{sync::Arc, time::Duration};

use bollard::container::LogOutput;
use ivynet_docker::{container::FullContainer, dockerapi::DockerClient};
use ivynet_signer::sign_utils::IvySigningError;
use kameo::{actor::ActorRef, Actor};
use tokio::{task::JoinSet, time};
use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::{
    ivy_machine::{IvyMachine, MachineIdentityError},
    telemetry::{dispatch::TelemetryDispatchHandle, ConfiguredAvs},
};

use super::dispatch::TelemetryMsg;

type LogListenerResult = Result<ListenerData, LogListenerError>;

/// Manager for a set of LogsListeners. This will spawn and manage the underlying listeners as
/// futures, and is made accessible via the `LogsListenerHandle`.
#[derive(Debug)]
pub struct LogsListenerManager {
    docker: DockerClient,
    machine: Arc<IvyMachine>,
    dispatcher: TelemetryDispatchHandle,
    listener_set: JoinSet<LogListenerResult>,
}

impl LogsListenerManager {
    pub fn new(
        docker: &DockerClient,
        machine: Arc<IvyMachine>,
        dispatcher: &TelemetryDispatchHandle,
    ) -> Self {
        Self {
            docker: docker.clone(),
            machine: machine.clone(),
            dispatcher: dispatcher.clone(),
            listener_set: JoinSet::new(),
        }
    }

    /// Add a listener to the manager as a future. The listener will be spawned and run in the
    /// background. The future will resolve to the container that the listener is listening to once
    /// the stream is closed for further handling, restarts, etc.
    pub async fn add_listener(
        &mut self,
        container: &FullContainer,
        node_data: &ConfiguredAvs,
    ) -> Result<(), LogListenerError> {
        let listener_data = ListenerData {
            container: container.clone(),
            node_data: node_data.clone(),
            machine: self.machine.clone(),
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

/// An individual instance of a LogListener, which listens to the logs of a single container and
/// sends them to the dispatcher. Has no associated handle, as each is a one-off actor.
#[derive(Debug)]
struct LogsListener {
    docker: DockerClient,
    dispatcher: TelemetryDispatchHandle,
    listener_data: ListenerData,
}

impl Actor for LogsListener {
    type Mailbox = kameo::mailbox::unbounded::UnboundedMailbox<Self>;

    async fn on_start(&mut self, _actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        match self.try_listen().await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListenerData {
    pub container: FullContainer,
    pub node_data: ConfiguredAvs,
    pub machine: Arc<IvyMachine>,
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
        let signed = self
            .listener_data
            .machine
            .sign_log(&self.listener_data.node_data.assigned_name, &log)?;
        match self.dispatcher.tell(TelemetryMsg::Log(signed)).await {
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
    SignatureError(#[from] IvySigningError),
    #[error("Unexpected error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    MachineIdentityErorr(#[from] MachineIdentityError),
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
