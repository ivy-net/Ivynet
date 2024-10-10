use crate::{
    avs::names::AvsName,
    grpc::{backend::backend_client::BackendClient, tonic::transport::Channel},
    wallet::IvyWallet,
};
use semver::Version;

#[derive(Debug)]
pub struct BackendMessenger {
    pub backend: BackendClient<Channel>,
    pub identity_wallet: IvyWallet,
}

impl BackendMessenger {
    pub fn new(backend: BackendClient<Channel>, identity_wallet: IvyWallet) -> Self {
        Self { backend, identity_wallet }
    }

    pub fn send_node_data_payload(
        &self,
        avs_name: AvsName,
        avs_version: Version,
        active_set: bool,
    ) {
        todo!()
    }
}
