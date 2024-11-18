use crate::{
    error::IvyError,
    grpc::{backend::backend_client::BackendClient, messages::NodeData, tonic::transport::Channel},
    wallet::IvyWallet,
};

#[derive(Debug)]
pub struct BackendMessenger {
    pub backend: BackendClient<Channel>,
    pub identity_wallet: IvyWallet,
}

impl BackendMessenger {
    pub fn new(backend: BackendClient<Channel>, identity_wallet: IvyWallet) -> Self {
        Self { backend, identity_wallet }
    }

    pub async fn send_node_data_payload(&mut self, _node_data: &NodeData) -> Result<(), IvyError> {
        todo!()
    }

    pub async fn delete_node_data_payload(&mut self) -> Result<(), IvyError> {
        todo!()
    }
}
