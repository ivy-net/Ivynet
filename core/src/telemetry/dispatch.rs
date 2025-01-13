use tonic::transport::Channel;

use crate::grpc::{
    backend::backend_client::BackendClient,
    messages::{SignedLog, SignedMetrics, SignedNodeData},
};

use super::ErrorChannelTx;

#[derive(Debug, Clone)]
pub enum TelemetryMsg {
    Metrics(SignedMetrics),
    NodeData(SignedNodeData),
    Log(SignedLog),
}

struct TelemetryDispatcher {
    rx: tokio::sync::mpsc::Receiver<TelemetryMsg>,
    error_tx: ErrorChannelTx,
    backend_client: BackendClient<Channel>,
}

impl TelemetryDispatcher {
    /// Run the telemetry dispatcher. This will listen for incoming telemetry messages and send
    /// them to the backend. If the backend is unreachable for any endpoint, it will send an error
    /// to the parent task error receiver. If the parent task reciever is closed, it will return an
    /// error back to the parent task.
    pub async fn run(&mut self) -> Result<(), TelemetryDispatchError> {
        while let Some(node_data) = self.rx.recv().await {
            let send_res = match node_data {
                TelemetryMsg::Metrics(metrics) => self.backend_client.metrics(metrics).await,
                TelemetryMsg::Log(log) => self.backend_client.logs(log).await,
                TelemetryMsg::NodeData(node_data) => self.backend_client.node_data(node_data).await,
            };
            if let Err(e) = send_res {
                let err = TelemetryDispatchError::TransportError(e);
                self.error_tx
                    .send(err.into())
                    .map_err(|_| TelemetryDispatchError::ErrorSendFailed)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct TelemetryDispatchHandle {
    tx: tokio::sync::mpsc::Sender<TelemetryMsg>,
}

impl Clone for TelemetryDispatchHandle {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone() }
    }
}

impl TelemetryDispatchHandle {
    pub async fn new(client: BackendClient<Channel>, error_tx: &ErrorChannelTx) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let error_tx = error_tx.clone();

        tokio::spawn(async move {
            let mut dispatcher = TelemetryDispatcher { rx, error_tx, backend_client: client };
            dispatcher.run().await
        });

        TelemetryDispatchHandle { tx }
    }

    pub async fn send(&self, msg: TelemetryMsg) -> Result<(), TelemetryDispatchError> {
        self.tx.send(msg).await.map_err(TelemetryDispatchError::DispatchError)
    }
    pub async fn send_metrics(&self, metrics: SignedMetrics) -> Result<(), TelemetryDispatchError> {
        self.send(TelemetryMsg::Metrics(metrics)).await
    }
    pub async fn send_node_data(
        &self,
        node_data: SignedNodeData,
    ) -> Result<(), TelemetryDispatchError> {
        self.send(TelemetryMsg::NodeData(node_data)).await
    }
    pub async fn send_log(&self, log: SignedLog) -> Result<(), TelemetryDispatchError> {
        self.send(TelemetryMsg::Log(log)).await
    }
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
    #[error("Telemetry send error: {0}")]
    TelemetrySendError(#[from] tokio::sync::mpsc::error::SendError<TelemetryMsg>),
}
