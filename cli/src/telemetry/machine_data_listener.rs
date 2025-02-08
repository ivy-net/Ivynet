use std::ops::Deref;

use ivynet_grpc::{messages::SignedMachineData, BackendMiddleware, Response, Status};
use kameo::{message::Message, Actor};
use tracing::debug;

use super::ErrorChannelTx;

#[derive(Debug, Clone)]
pub struct MachineDataMonitorHandle<B: BackendMiddleware>(
    kameo::actor::ActorRef<MachineDataMonitor<B>>,
);

impl<B: BackendMiddleware> MachineDataMonitorHandle<B> {
    pub fn new(backend_client: B, _error_tx: ErrorChannelTx) -> Self {
        Self(kameo::actor::spawn(MachineDataMonitor::new(backend_client, _error_tx)))
    }

    pub async fn tell_send_machine_data(&self, machine_data: SignedMachineData) {
        self.0.tell(MachineDataMsg::MachineData(machine_data));
    }

    pub async fn ask_send_machine_data(
        &self,
        machine_data: SignedMachineData,
    ) -> Result<Response<()>, MachineDataMonitorError> {
        debug!("ASK | {:?}", machine_data);
        self.0
            .ask(MachineDataMsg::MachineData(machine_data))
            .await
            .map_err(MachineDataMonitorError::from)
    }
}

impl<B: BackendMiddleware> Deref for MachineDataMonitorHandle<B> {
    type Target = kameo::actor::ActorRef<MachineDataMonitor<B>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Actor, Debug)]
pub struct MachineDataMonitor<B: BackendMiddleware> {
    backend_client: B,
    _error_tx: ErrorChannelTx,
}

pub enum MachineDataMsg {
    MachineData(SignedMachineData),
}

impl<B: BackendMiddleware> MachineDataMonitor<B> {
    pub fn new(backend_client: B, _error_tx: ErrorChannelTx) -> Self {
        Self { _error_tx, backend_client }
    }
}

impl<B: BackendMiddleware> Message<MachineDataMsg> for MachineDataMonitor<B> {
    type Reply = Result<Response<()>, Status>;

    async fn handle(
        &mut self,
        msg: MachineDataMsg,
        _: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            MachineDataMsg::MachineData(machine_data) => {
                self.backend_client.machine_data(machine_data).await
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MachineDataMonitorError {
    #[error("Internal server error: {0}")]
    InternalServerError(Status),

    #[error("Failed to send machine data: {0}")]
    FailedToSendMachineData(#[from] kameo::error::SendError<MachineDataMsg, ivynet_grpc::Status>),
}
