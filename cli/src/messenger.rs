use ivynet_core::grpc::{backend::backend_client::BackendClient, tonic::transport::Channel};

pub struct BackendMessenger {
    pub backend: BackendClient<Channel>,
}
