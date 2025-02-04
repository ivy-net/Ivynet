use std::ops::Deref;

use ivynet_grpc::{messages::SignedNodeDataV2, BackendMiddleware, Response, Status};
use kameo::{message::Message, Actor};
use tracing::debug;

use super::ErrorChannelTx;

#[derive(Debug, Clone)]
pub struct NodeDataMonitorHandle<B: BackendMiddleware>(kameo::actor::ActorRef<NodeDataMonitor<B>>);

impl<B: BackendMiddleware> NodeDataMonitorHandle<B> {
    pub fn new(backend_client: B, _error_tx: ErrorChannelTx) -> Self {
        Self(kameo::actor::spawn(NodeDataMonitor::new(backend_client, _error_tx)))
    }

    pub async fn tell_send_node_data(&self, node_data: SignedNodeDataV2) {
        self.0.tell(NodeDataMsg::NodeData(node_data));
    }

    pub async fn ask_send_node_data(
        &self,
        node_data: SignedNodeDataV2,
    ) -> Result<Response<()>, NodeDataMonitorError> {
        debug!("ASK | {:?}", node_data);
        self.0.ask(NodeDataMsg::NodeData(node_data)).await.map_err(NodeDataMonitorError::from)
    }
}

impl<B: BackendMiddleware> Deref for NodeDataMonitorHandle<B> {
    type Target = kameo::actor::ActorRef<NodeDataMonitor<B>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Actor, Debug)]
pub struct NodeDataMonitor<B: BackendMiddleware> {
    backend_client: B,
    _error_tx: ErrorChannelTx,
}

pub enum NodeDataMsg {
    NodeData(SignedNodeDataV2),
}

impl<B: BackendMiddleware> NodeDataMonitor<B> {
    pub fn new(backend_client: B, _error_tx: ErrorChannelTx) -> Self {
        Self { _error_tx, backend_client }
    }
}

impl<B: BackendMiddleware> Message<NodeDataMsg> for NodeDataMonitor<B> {
    type Reply = Result<Response<()>, Status>;

    async fn handle(
        &mut self,
        msg: NodeDataMsg,
        _: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            NodeDataMsg::NodeData(node_data) => self.backend_client.node_data_v2(node_data).await,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeDataMonitorError {
    #[error("Internal server error: {0}")]
    InternalServerError(Status),

    #[error("Failed to send node data: {0}")]
    FailedToSendNodeData(#[from] kameo::error::SendError<NodeDataMsg, ivynet_grpc::Status>),
}

#[cfg(test)]
mod node_data_monitor_test {
    // use std::str::FromStr;

    // use crate::config::IvyConfig;

    // use super::*;
    // use ivynet_grpc::{client::create_channel, messages::NodeData, BackendClientMiddleware};
    // use ivynet_signer::{sign_utils::sign_node_data, IvyWallet};
    // use sqlx::PgPool;

    // async fn backend_client_fixture() -> BackendClientMiddleware {
    //     let config = IvyConfig::default();
    //     let backend_url = config.get_server_url().unwrap();
    //     let backend_ca = config.get_server_ca();
    //     let backend_ca = if config.get_server_ca().is_empty() { None } else { Some(backend_ca) };
    //     let channel = create_channel(backend_url, backend_ca).await.unwrap();
    //     BackendClientMiddleware::from_channel(channel)
    // }

    // #[ignore]
    // #[tokio::test]
    // async fn test_failed_node_data_send() {
    //     let (tx, mut rx) = tokio::sync::broadcast::channel(1);
    //     let client = backend_client_fixture().await;

    //     let test_signer = IvyWallet::new();

    //     let node_data = NodeData {
    //         name: "failed_send_node".to_string(),
    //         node_type: Some("test".to_string()),
    //         manifest: Some("test".to_string()),
    //         metrics_alive: Some(false),
    //         node_running: Some(true),
    //     };

    //     let node_data_signature = sign_node_data(&node_data, &test_signer).unwrap();

    //     let signed_node_data = SignedNodeData {
    //         machine_id: uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, "test_machine".as_bytes())
    //             .into(),
    //         signature: node_data_signature.to_vec(),
    //         node_data: Some(node_data),
    //     };

    //     let node_data_msg = NodeDataMsg::NodeData(signed_node_data);

    //     let actor = NodeDataMonitor::new(client, tx);
    //     let actor_ref = kameo::actor::spawn(actor);

    //     let resp = actor_ref.ask(node_data_msg).await;
    //     assert!(resp.is_err())
    // }

    // #[ignore]
    // #[sqlx::test(
    //     migrations = "../migrations",
    //     fixtures(
    //         "../../../fixtures/avs_version_hashes.sql",
    //         "../../../fixtures/node_data_versions.sql",
    //         "../../../fixtures/test_organization.sql",
    //         "../../../fixtures/test_client.sql",
    //         "../../../fixtures/test_machine.sql",
    //     )
    // )]
    // async fn test_successful_node_data_send(pool: PgPool) {
    //     let (tx, mut rx) = tokio::sync::broadcast::channel(1);
    //     let client = backend_client_fixture().await;

    //     let test_signer = IvyWallet::new();

    //     let node_data = NodeData {
    //         name: "successful_send_node".to_string(),
    //         node_type: Some("test".to_string()),
    //         manifest: Some("test".to_string()),
    //         metrics_alive: Some(false),
    //         node_running: Some(true),
    //     };

    //     let node_data_signature = sign_node_data(&node_data, &test_signer).unwrap();

    //     let signed_node_data = SignedNodeData {
    //         machine_id: uuid::Uuid::from_str("0cdaf7b9-1824-44b9-9e96-26ed64906f87")
    //             .unwrap()
    //             .into(),
    //         signature: node_data_signature.to_vec(),
    //         node_data: Some(node_data),
    //     };

    //     let node_data_msg = NodeDataMsg::NodeData(signed_node_data);

    //     let actor = NodeDataMonitor::new(client, tx);
    //     let actor_ref = kameo::actor::spawn(actor);

    //     let resp = actor_ref.ask(node_data_msg).await;
    //     assert!(resp.is_ok())
    // }
}
