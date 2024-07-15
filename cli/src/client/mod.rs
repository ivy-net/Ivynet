use ivynet_core::grpc::tonic::transport::Channel;

use self::{avs::AvsClient, operator::OperatorClient};

pub mod avs;
pub mod operator;

pub struct IvynetClient {
    avs_client: AvsClient,
    operator_client: OperatorClient,
}

/// GRPC client wrapper constructing for AVS and Operator client types.
impl IvynetClient {
    pub fn new(avs_client: AvsClient, operator_client: OperatorClient) -> Self {
        Self { avs_client, operator_client }
    }

    pub fn from_channel(channel: Channel) -> Self {
        Self {
            avs_client: AvsClient::new(channel.clone()),
            operator_client: OperatorClient::new(channel),
        }
    }

    pub fn avs(&self) -> &AvsClient {
        &self.avs_client
    }

    pub fn avs_mut(&mut self) -> &mut AvsClient {
        &mut self.avs_client
    }

    pub fn operator(&self) -> &OperatorClient {
        &self.operator_client
    }

    pub fn operator_mut(&mut self) -> &mut OperatorClient {
        &mut self.operator_client
    }
}
