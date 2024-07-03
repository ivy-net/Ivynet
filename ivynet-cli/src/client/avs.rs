use ivynet_core::{
    error::IvyError,
    grpc::{
        ivynet_api::{
            ivy_daemon_avs::{
                avs_client::AvsClient as AvsClientRaw, AvsInfoRequest, AvsInfoResponse, OptinRequest, OptoutRequest,
                SetAvsRequest, StartRequest, StopRequest,
            },
            ivy_daemon_types::RpcResponse,
        },
        tonic::{transport::Channel, Request, Response},
    },
};

pub struct AvsClient(AvsClientRaw<Channel>);

impl AvsClient {
    pub fn new(channel: Channel) -> Self {
        Self(AvsClientRaw::new(channel))
    }

    pub async fn avs_info(&mut self) -> Result<Response<AvsInfoResponse>, IvyError> {
        let request = Request::new(AvsInfoRequest {});
        let response = self.0.avs_info(request).await?;
        Ok(response)
    }

    pub async fn opt_in(&mut self) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(OptinRequest {});
        let response = self.0.opt_in(request).await?;
        Ok(response)
    }

    pub async fn opt_out(&mut self) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(OptoutRequest {});
        let response = self.0.opt_out(request).await?;
        Ok(response)
    }

    pub async fn start(
        &mut self,
        avs: Option<String>,
        chain: Option<String>,
    ) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(StartRequest { avs, chain });
        let response = self.0.start(request).await?;
        Ok(response)
    }

    pub async fn stop(&mut self) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(StopRequest {});
        let response = self.0.stop(request).await?;
        Ok(response)
    }

    pub async fn set_avs(&mut self, avs: String, chain: String) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(SetAvsRequest { avs, chain });
        let response = self.0.set_avs(request).await?;
        Ok(response)
    }
}
