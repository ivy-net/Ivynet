use ivynet_core::{
    avs::build_avs_provider,
    config::IvyConfig,
    docker::dockercmd::DockerCmd,
    fluentd::log_server::serve_log_server,
    grpc::{
        backend::backend_client::BackendClient,
        client::{create_channel, Uri},
        ivynet_api::{
            ivy_daemon_avs::avs_server::AvsServer,
            ivy_daemon_operator::operator_server::OperatorServer,
        },
        server::{Endpoint, Server},
    },
    keychain::{KeyType, Keychain},
    messenger::BackendMessenger,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::{error::Error, rpc::ivynet::IvynetService, telemetry};

pub async fn serve(
    avs: Option<String>,
    chain: Option<String>,
    config: &IvyConfig,
    server_url: Uri,
    server_ca: Option<String>,
    no_backend: bool,
) -> Result<(), Error> {
    let sock = Endpoint::Path(config.uds_dir());

    let keychain = Keychain::default();
    let keyname = keychain.select_key(KeyType::Ecdsa)?;
    let keyfile_pw = dialoguer::Password::new()
        .with_prompt("Input the password for your stored Operator ECDSA keyfile")
        .interact()?;
    let key = keychain.load(keyname, &keyfile_pw)?;

    // TODO: Shoud this log an error if the wallet is not found instead of just closing?
    if let Some(wallet) = key.get_wallet_owned() {
        let connection_wallet = config.identity_wallet()?;
        let backend_client = BackendClient::new(
            create_channel(ivynet_core::grpc::client::Source::Uri(server_url), server_ca).await?,
        );
        let messenger = BackendMessenger::new(backend_client.clone(), connection_wallet.clone());

        // Avs Service

        // TODO: This should default to local instead of holesky?
        let chain = chain.unwrap_or_else(|| "holesky".to_string());
        let avs_provider = build_avs_provider(
            avs.as_deref(),
            &chain,
            config,
            Some(wallet.clone()),
            Some(keyfile_pw.to_owned()),
            Some(messenger),
        )
        .await?;
        let ivynet_inner = Arc::new(RwLock::new(avs_provider));

        ///////////////////
        // Logging
        ///////////////////

        // Set logging directory
        let fluentd_path = config.get_dir().join("fluentd");
        std::env::set_var("FLUENTD_PATH", fluentd_path.to_str().unwrap());
        info!("Serving local logs at {:?}", fluentd_path);
        // Start the container
<<<<<<< HEAD
        DockerCmd::new().args(["up", "--build"]).current_dir(&fluentd_path).spawn()?;
=======
        DockerCmd::new().args(["up", "-d", "--build"]).current_dir(&fluentd_path).spawn()?;
>>>>>>> dev
        info!("Fluentd logging container started");

        ///////////////////
        // GRPC
        ///////////////////

        // NOTE: Due to limitations with Prost / GRPC, we create a new server with a
        // reference-counted handle to the inner type for each server, as opposed to cloning
        // / being able to clone the outer service.
        let avs_server = AvsServer::new(IvynetService::new(ivynet_inner.clone()));
        let operator_server = OperatorServer::new(IvynetService::new(ivynet_inner.clone()));

        let server = Server::new(avs_server, None, None).add_service(operator_server);
        if no_backend {
<<<<<<< HEAD
            tokio::select! {
                ret = server.serve(sock) => { error!("Local server error {ret:?}") },
            }
        } else {
            let connection_wallet = config.identity_wallet()?;
            tokio::select! {
                ret = server.serve(sock) => { error!("Local server error {ret:?}") },
=======
            tokio::select! {
                ret = server.serve(sock) => { error!("Local server error {ret:?}") },
            }
        } else {
            tokio::select! {
                ret = server.serve(sock) => { error!("Local server error {ret:?}") },
>>>>>>> dev
                ret = serve_log_server(backend_client.clone(), connection_wallet.clone()) => { error!("Log server error {ret:?}") }
                ret = telemetry::listen(ivynet_inner, backend_client, connection_wallet) => { error!("Telemetry listener error {ret:?}") }
            }
        }
    }
    Ok(())
}
