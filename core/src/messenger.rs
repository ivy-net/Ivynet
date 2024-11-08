use crate::{
    avs::names::AvsName,
    error::IvyError,
    grpc::{backend::backend_client::BackendClient, messages::NodeData, tonic::transport::Channel},
    wallet::IvyWallet,
};
use ethers::types::Address;

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
        // TODO: To be removed. Serve is not talking to backend anymore
        Ok(())
    }

    pub async fn delete_node_data_payload(
        &mut self,
        _operator_id: Address,
        _avs_name: AvsName,
    ) -> Result<(), IvyError> {
        // TODO: This is to be removed now. Serve is not talking with the backend itself.
        Ok(())
    }
}
