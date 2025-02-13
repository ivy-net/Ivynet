use std::ops::Deref;

use ivynet_grpc::{
    backend::backend_client::BackendClient,
    messages::{SignedLog, SignedMachineData, SignedMetrics, SignedNodeDataV2},
    tonic::{self, transport::Channel},
};
use kameo::{message::Message, Actor};
use tracing::error;

use super::ErrorChannelTx;

#[derive(Debug, Clone)]
pub enum TelemetryMsg {
    SignedNodeData(SignedNodeDataV2),
    Metrics(SignedMetrics),
    Log(SignedLog),
    SignedMachineData(SignedMachineData),
}

#[derive(Debug, Clone)]
pub struct TelemetryDispatchHandle(kameo::actor::ActorRef<TelemetryDispatch>);

impl TelemetryDispatchHandle {
    pub fn new(backend_client: BackendClient<Channel>, error_tx: ErrorChannelTx) -> Self {
        Self(kameo::actor::spawn(TelemetryDispatch::new(backend_client, error_tx)))
    }
}

impl Deref for TelemetryDispatchHandle {
    type Target = kameo::actor::ActorRef<TelemetryDispatch>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Actor, Debug)]
#[actor(name = "TelemetryDispatch", mailbox = bounded(64))]
pub struct TelemetryDispatch {
    pub error_tx: ErrorChannelTx,
    pub backend_client: BackendClient<Channel>,
}

impl TelemetryDispatch {
    pub fn new(backend_client: BackendClient<Channel>, error_tx: ErrorChannelTx) -> Self {
        Self { error_tx, backend_client }
    }
}

impl Message<TelemetryMsg> for TelemetryDispatch {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: TelemetryMsg,
        _: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        let res = match msg {
            TelemetryMsg::Metrics(metrics) => self.backend_client.metrics(metrics).await,
            TelemetryMsg::Log(log) => self.backend_client.logs(log).await,
            TelemetryMsg::SignedNodeData(node_data) => {
                self.backend_client.node_data_v2(node_data).await
            }
            TelemetryMsg::SignedMachineData(machine_data) => {
                self.backend_client.machine_data(machine_data).await
            }
        };
        match res {
            Ok(_) => {}
            Err(e) => {
                error!("Telemetry dispatch error: {:?}", e);
            }
        }
    }
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum TelemetryDispatchError {
    #[error("Failed to get telemetry error. The channel has been previously closed.")]
    ChannelClosed,
    #[error(transparent)]
    TransportError(tonic::Status),
    #[error("Failed to send error to the parent task.")]
    ErrorSendFailed,
}
