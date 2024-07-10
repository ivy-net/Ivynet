use ivynet_core::grpc::ivynet_api::ivy_daemon_avs::avs_server::AvsServer;
use ivynet_core::grpc::ivynet_api::ivy_daemon_operator::operator_server::OperatorServer;
use ivynet_core::grpc::server::Endpoint;
use ivynet_core::{
    avs::build_avs_provider, config::IvyConfig, grpc::server::Server, wallet::IvyWallet,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{error::Error, rpc::ivynet::IvynetService};

pub async fn serve(
    avs: Option<String>,
    chain: Option<String>,
    config: &IvyConfig,
    keyfile_pw: &str,
) -> Result<(), Error> {
    let sock = Endpoint::Path(config.uds_dir());

    // Keystore load
    let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), keyfile_pw)?;

    // Avs Service
    // TODO: This should default to local instead of holesky?
    let chain = chain.unwrap_or_else(|| "holesky".to_string());
    let avs_provider = build_avs_provider(
        avs.as_deref(),
        &chain,
        config,
        Some(wallet),
        Some(keyfile_pw.to_owned()),
    )
    .await?;
    let ivynet_inner = Arc::new(RwLock::new(avs_provider));

    // NOTE: Due to limitations with Prost / GRPC, we create a new server with a reference-counted
    // handle to the inner type for each server, as opposed to cloning / being able to clone the
    // outer service.
    let avs_server = AvsServer::new(IvynetService::new(ivynet_inner.clone()));
    let operator_server = OperatorServer::new(IvynetService::new(ivynet_inner.clone()));

    let server = Server::new(avs_server, None, None).add_service(operator_server);

    println!("Starting the IvyNet service at {}...", sock);

    server.serve(sock).await.expect("Failed to start IvyNet Service.");
    Ok(())
}
