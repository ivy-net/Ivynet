use ivynet_core::grpc::tonic::transport::Channel;

use self::avs::AvsClient;

pub mod avs;

pub struct IvynetClient {
    avs_client: AvsClient,
}

impl IvynetClient {
    pub fn new(avs_client: AvsClient) -> Self {
        Self { avs_client }
    }

    pub fn from_channel(channel: Channel) -> Self {
        Self { avs_client: AvsClient::new(channel) }
    }

    pub fn avs(&self) -> &AvsClient {
        &self.avs_client
    }

    pub fn avs_mut(&mut self) -> &mut AvsClient {
        &mut self.avs_client
    }
}
