pub mod client;
pub mod server;

pub mod backend {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("backend");
}

pub mod backend_events {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("backend_events");
}

pub mod messages {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("messages");
}

pub mod node_data {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("node_data");
}

use backend::backend_client::BackendClient;
use messages::RegistrationCredentials;
use tonic::transport::Channel;

pub use tonic::{self, async_trait, Response, Status};

#[derive(Debug, Clone)]
pub struct BackendClientMiddleware(backend::backend_client::BackendClient<Channel>);

#[async_trait]
pub trait BackendMiddleware: Clone + Send + Sync + 'static {
    fn new(client: BackendClient<Channel>) -> Self;

    fn from_channel(channel: Channel) -> Self;

    async fn register(
        &mut self,
        request: impl tonic::IntoRequest<RegistrationCredentials> + Send,
    ) -> std::result::Result<Response<()>, Status>;

    async fn metrics(&mut self, request: messages::SignedMetrics) -> Result<Response<()>, Status>;

    async fn node_data(
        &mut self,
        request: messages::SignedNodeData,
    ) -> Result<Response<()>, Status>;

    async fn node_data_v2(
        &mut self,
        request: messages::SignedNodeDataV2,
    ) -> Result<Response<()>, Status>;

    async fn logs(&mut self, request: messages::SignedLog) -> Result<Response<()>, Status>;

    async fn node_type_queries(
        &mut self,
        request: messages::NodeTypeQueries,
    ) -> Result<Response<messages::NodeTypes>, Status>;

    async fn name_change(
        &mut self,
        request: messages::SignedNameChange,
    ) -> Result<Response<()>, Status>;
}

impl From<BackendClient<Channel>> for BackendClientMiddleware {
    fn from(client: BackendClient<Channel>) -> Self {
        Self(client)
    }
}

#[async_trait]
impl BackendMiddleware for BackendClientMiddleware {
    fn new(client: BackendClient<Channel>) -> Self {
        Self(client)
    }

    fn from_channel(channel: Channel) -> Self {
        Self(BackendClient::new(channel))
    }

    async fn register(
        &mut self,
        request: impl tonic::IntoRequest<RegistrationCredentials> + Send,
    ) -> std::result::Result<Response<()>, Status> {
        self.0.register(request).await
    }

    async fn metrics(&mut self, request: messages::SignedMetrics) -> Result<Response<()>, Status> {
        self.0.metrics(request).await
    }

    async fn node_data(
        &mut self,
        request: messages::SignedNodeData,
    ) -> Result<Response<()>, Status> {
        self.0.node_data(request).await
    }

    async fn node_data_v2(
        &mut self,
        request: messages::SignedNodeDataV2,
    ) -> Result<Response<()>, Status> {
        self.0.node_data_v2(request).await
    }

    async fn logs(&mut self, request: messages::SignedLog) -> Result<Response<()>, Status> {
        self.0.logs(request).await
    }

    async fn node_type_queries(
        &mut self,
        request: messages::NodeTypeQueries,
    ) -> Result<Response<messages::NodeTypes>, Status> {
        self.0.node_type_queries(request).await
    }

    async fn name_change(
        &mut self,
        request: messages::SignedNameChange,
    ) -> Result<Response<()>, Status> {
        self.0.name_change(request).await
    }
}

#[derive(Debug, Clone)]
pub struct BackendClientMock;

impl BackendClientMock {
    #[allow(dead_code)]
    async fn wait(&self, ms: u64) -> Result<(), Status> {
        tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
        Ok(())
    }
}

#[async_trait]
impl BackendMiddleware for BackendClientMock {
    fn new(_client: BackendClient<Channel>) -> Self {
        Self
    }

    fn from_channel(_channel: Channel) -> Self {
        Self
    }

    async fn register(
        &mut self,
        _request: impl tonic::IntoRequest<RegistrationCredentials> + Send,
    ) -> std::result::Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn metrics(&mut self, _request: messages::SignedMetrics) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn node_data(
        &mut self,
        _request: messages::SignedNodeData,
    ) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn node_data_v2(
        &mut self,
        _request: messages::SignedNodeDataV2,
    ) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn logs(&mut self, _request: messages::SignedLog) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn node_type_queries(
        &mut self,
        _request: messages::NodeTypeQueries,
    ) -> Result<Response<messages::NodeTypes>, Status> {
        let node_type_resp = messages::NodeType {
            container_name: "david-byrne".to_string(),
            node_type: "talking-heads".to_string(),
        };
        Ok(Response::new(messages::NodeTypes { node_types: vec![node_type_resp] }))
    }

    async fn name_change(
        &mut self,
        _request: messages::SignedNameChange,
    ) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }
}
