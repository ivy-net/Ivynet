use ivynet_core::{
    avs::build_avs_provider,
    config::IvyConfig,
    grpc::{
        backend::backend_client::BackendClient,
        client::{create_channel, Uri},
        ivynet_api::{
            ivy_daemon_avs::avs_server::AvsServer,
            ivy_daemon_operator::operator_server::OperatorServer,
        },
        server::{Endpoint, Server},
    },
    wallet::IvyWallet,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;

use crate::{error::Error, rpc::ivynet::IvynetService, telemetry};

pub async fn serve(
    avs: Option<String>,
    chain: Option<String>,
    config: &IvyConfig,
    keyfile_pw: &str,
    server_url: Uri,
    server_ca: Option<String>,
    no_backend: bool,
) -> Result<(), Error> {
    let sock = Endpoint::Path(config.uds_dir());

    // Keystore load
    let wallet = IvyWallet::from_keystore(config.default_ecdsa_keyfile.clone(), keyfile_pw)?;

    // Avs Service
    // TODO: This should default to local instead of holesky?
    let chain = chain.unwrap_or_else(|| "holesky".to_string());
    let avs_provider = build_avs_provider(
        avs.as_deref(),
        &chain,
        config,
        Some(wallet.clone()),
        Some(keyfile_pw.to_owned()),
    )
    .await?;
    let ivynet_inner = Arc::new(RwLock::new(avs_provider));

    // NOTE: Due to limitations with Prost / GRPC, we create a new server with a reference-counted
    // handle to the inner type for each server, as opposed to cloning / being able to clone the
    // outer service.
    let avs_server = AvsServer::new(IvynetService::new(ivynet_inner.clone()));
    let operator_server = OperatorServer::new(IvynetService::new(ivynet_inner.clone()));
    let backend_client = BackendClient::new(
        create_channel(ivynet_core::grpc::client::Source::Uri(server_url), server_ca).await?,
    );

    let server = Server::new(avs_server, None, None).add_service(operator_server);

    println!("Starting the IvyNet service at {}...", sock);

    if no_backend {
        server.serve(sock).await?;
    } else {
        let connection_wallet = config.identity_wallet()?;
        tokio::select! {
            ret = server.serve(sock) => { error!("Local server error {ret:?}") },
            ret = telemetry::listen(ivynet_inner, backend_client, connection_wallet) => { error!("Telemetry listener error {ret:?}") }
        }
    }

    Ok(())
}
