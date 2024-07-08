use ivynet_core::{
    error::IvyError,
    grpc::{
        ivynet_api::ivy_daemon_operator::{
            operator_client::OperatorClient as OperatorClientRaw, DelegatableSharesRequest,
            DelegatableSharesResponse, OperatorDetailsRequest, OperatorDetailsResponse,
            OperatorSharesRequest, OperatorSharesResponse,
        },
        tonic::{transport::Channel, Request, Response},
    },
};

pub struct OperatorClient(OperatorClientRaw<Channel>);

impl OperatorClient {
    pub fn new(channel: Channel) -> Self {
        Self(OperatorClientRaw::new(channel))
    }

    pub async fn get_operator_details(
        &mut self,
    ) -> Result<Response<OperatorDetailsResponse>, IvyError> {
        let request = Request::new(OperatorDetailsRequest {});
        let response = self.0.get_operator_details(request).await?;
        Ok(response)
    }

    pub async fn get_operator_shares(
        &mut self,
    ) -> Result<Response<OperatorSharesResponse>, IvyError> {
        let request = Request::new(OperatorSharesRequest {});
        let response = self.0.get_operator_shares(request).await?;
        Ok(response)
    }

    pub async fn get_delegatable_shares(
        &mut self,
        address: Option<String>,
    ) -> Result<Response<DelegatableSharesResponse>, IvyError> {
        let request = Request::new(DelegatableSharesRequest { address });
        let response = self.0.get_delegatable_shares(request).await?;
        Ok(response)
    }
}
