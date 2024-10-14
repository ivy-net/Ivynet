use crate::{
    avs::names::AvsName,
    error::IvyError,
    grpc::{
        backend::backend_client::BackendClient,
        messages::{NodeData, SignedDeleteNodeData, SignedNodeData},
        tonic::transport::Channel,
    },
    signature::{sign_delete_node_data, sign_node_data},
    wallet::IvyWallet,
};
use ethers::types::Address;
use tonic::Request;

#[derive(Debug)]
pub struct BackendMessenger {
    pub backend: BackendClient<Channel>,
    pub identity_wallet: IvyWallet,
}

impl BackendMessenger {
    pub fn new(backend: BackendClient<Channel>, identity_wallet: IvyWallet) -> Self {
        Self { backend, identity_wallet }
    }

    pub async fn send_node_data_payload(&mut self, node_data: &NodeData) -> Result<(), IvyError> {
        let signature = sign_node_data(node_data, &self.identity_wallet)?;

        let signed_node_data =
            SignedNodeData { signature: signature.to_vec(), node_data: Some(node_data.clone()) };

        let request = Request::new(signed_node_data);
        self.backend.node_data(request).await?;
        Ok(())
    }

    pub async fn delete_node_data_payload(
        &mut self,
        operator_id: Address,
        avs_name: AvsName,
    ) -> Result<(), IvyError> {
        let signature =
            sign_delete_node_data(operator_id, avs_name.to_string(), &self.identity_wallet)?;

        let signed_node_data = SignedDeleteNodeData {
            signature: signature.to_vec(),
            operator_id: operator_id.as_bytes().to_vec(),
            avs_name: avs_name.to_string(),
        };

        let request = Request::new(signed_node_data);
        self.backend.delete_node_data(request).await?;
        Ok(())
    }
}
