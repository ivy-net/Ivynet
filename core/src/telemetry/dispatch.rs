use tonic::transport::{Channel, Uri};

use crate::{
    error::IvyError,
    grpc::{
        backend::backend_client::BackendClient,
        messages::{SignedMetrics, SignedNodeData},
    },
};

#[derive(Debug, Clone)]
pub enum TelemetryMsg {
    UpdateNodeData(SignedNodeData),
    DeleteNodeData(SignedNodeData),
    Metrics(SignedMetrics),
}

pub struct TelemetryDispatcher {
    rx: tokio::sync::mpsc::Receiver<TelemetryMsg>,
    error_tx: tokio::sync::broadcast::Sender<TelemetryDispatchError>,
    backend_client: BackendClient<Channel>,
}

// TODO: We don't currently await the joinhandle from this task anywhere in the parent thread.
// Consider an initialization method which returns both the handle to the dispatcher and the
// task joinhandle to the parent thread.

impl TelemetryDispatcher {
    pub async fn run(&mut self) -> Result<(), TelemetryDispatchError> {
        while let Some(node_data) = self.rx.recv().await {
            let send_res = match node_data {
                TelemetryMsg::UpdateNodeData(node_data) => {
                    self.backend_client.update_node_data(node_data).await
                }
                TelemetryMsg::DeleteNodeData(node_data) => {
                    self.backend_client.delete_node_data(node_data).await
                }
                TelemetryMsg::Metrics(metrics) => self.backend_client.metrics(metrics).await,
            };
            if let Err(e) = send_res {
                let err = TelemetryDispatchError::TransportError(e);
                self.error_tx.send(err).map_err(|_| TelemetryDispatchError::ErrorSendFailed)?;
            }
        }
        Ok(())
    }
}

pub struct TelemetryDispatchHandle {
    tx: tokio::sync::mpsc::Sender<TelemetryMsg>,
    pub error_rx: tokio::sync::broadcast::Receiver<TelemetryDispatchError>,
}

impl TelemetryDispatchHandle {
    pub async fn send(&self, msg: TelemetryMsg) -> Result<(), IvyError> {
        self.tx.send(msg).await.map_err(IvyError::from)
    }
    pub async fn from_client(client: BackendClient<Channel>) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let (error_tx, error_rx) = tokio::sync::broadcast::channel(16);

        tokio::spawn(async move {
            let mut dispatcher = TelemetryDispatcher { rx, error_tx, backend_client: client };
            dispatcher.run().await
        });

        TelemetryDispatchHandle { tx, error_rx }
    }
    pub async fn send_node_data(&self, node_data: SignedNodeData) -> Result<(), IvyError> {
        self.send(TelemetryMsg::UpdateNodeData(node_data)).await
    }
    pub async fn delete_node_data(&self, node_data: SignedNodeData) -> Result<(), IvyError> {
        self.send(TelemetryMsg::DeleteNodeData(node_data)).await
    }
    pub async fn send_metrics(&self, metrics: SignedMetrics) -> Result<(), IvyError> {
        self.send(TelemetryMsg::Metrics(metrics)).await
    }
}

/// Creates a handle to telemetry dispatch actor. Actor sends telemetry messages to the backend in
/// its own task.
pub async fn create_telemetry_dispatch(
    backend_url: Uri,
    backend_ca: Option<String>,
) -> TelemetryDispatchHandle {
    // TODO: Channel size is currently limited to 256. Consider unbounded channel.
    let backend_client = BackendClient::new(
        crate::grpc::client::create_channel(
            crate::grpc::client::Source::Uri(backend_url),
            backend_ca,
        )
        .await
        .expect("Cannot create channel"),
    );
    TelemetryDispatchHandle::from_client(backend_client).await
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum TelemetryDispatchError {
    #[error("Failed to get telemetry error. The channel has been previously closed.")]
    ChannelClosed,
    #[error(transparent)]
    DispatchError(tokio::sync::mpsc::error::SendError<TelemetryMsg>),
    #[error(transparent)]
    TransportError(tonic::Status),
    #[error("Failed to send error to the parent task.")]
    ErrorSendFailed,
}
