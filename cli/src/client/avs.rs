use ivynet_core::{
    error::IvyError,
    grpc::{
        ivynet_api::{
            ivy_daemon_avs::{
                avs_client::AvsClient as AvsClientRaw, AttachRequest, AvsInfoRequest,
                AvsInfoResponse, RegisterRequest, SelectAvsRequest, StartRequest, StopRequest,
                UnregisterRequest,
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

    pub async fn register(&mut self) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(RegisterRequest {});
        let response = self.0.register(request).await?;
        Ok(response)
    }

    pub async fn unregister(&mut self) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(UnregisterRequest {});
        let response = self.0.unregister(request).await?;
        Ok(response)
    }

    pub async fn start(
        &mut self,
        avs: Option<String>,
        chain: Option<String>,
    ) -> Result<Response<RpcResponse>, IvyError> {
        if let (Some(avs), Some(chain)) = (avs.clone(), chain.clone()) {
            let request = Request::new(SelectAvsRequest { avs, chain });
            let _ = self.0.select_avs(request).await?;
        }

        let request = Request::new(StartRequest { avs, chain });
        let response = self.0.start(request).await?;
        Ok(response)
    }

    pub async fn stop(&mut self) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(StopRequest {});
        let response = self.0.stop(request).await?;
        Ok(response)
    }

    pub async fn select_avs(
        &mut self,
        avs: String,
        chain: String,
    ) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(SelectAvsRequest { avs, chain });
        let response = self.0.select_avs(request).await?;
        Ok(response)
    }

    pub async fn attach(
        &mut self,
        avs: Option<String>,
        chain: Option<String>,
    ) -> Result<Response<RpcResponse>, IvyError> {
        if let (Some(avs), Some(chain)) = (avs.clone(), chain.clone()) {
            let request = Request::new(SelectAvsRequest { avs, chain });
            let _ = self.0.select_avs(request).await?;
        }

        let request = Request::new(AttachRequest { avs, chain });
        let response = self.0.attach(request).await?;

        Ok(response)
    }
}
