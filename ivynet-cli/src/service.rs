use dialoguer::Password;
use ivynet_core::grpc::ivynet_api::ivy_daemon_avs::avs_server::AvsServer;
use ivynet_core::grpc::ivynet_api::ivy_daemon_operator::operator_server::OperatorServer;
use ivynet_core::{
    avs::build_avs_provider, config::IvyConfig, grpc::server::Server,
    wallet::IvyWallet,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{error::Error, rpc::ivynet::IvynetService};

const DEFAULT_PORT: u16 = 55501;

pub async fn serve(
    avs: Option<String>,
    chain: Option<String>,
    port: Option<u16>,
    config: &IvyConfig,
    keyfile_pw: &str,
) -> Result<(), Error> {
    let port = port.unwrap_or(DEFAULT_PORT);

    // Keystore load
    let password: String = Password::new()
        .with_prompt("Input the password for your stored keyfile")
        .interact()?;
    let wallet = IvyWallet::from_keystore(
        config.default_private_keyfile.clone(),
        &password,
    )?;

    // Avs Service
    // TODO: This should default to local instead of holesky?
    let chain = chain.unwrap_or_else(|| "holesky".to_string());
    let avs_provider = build_avs_provider(
        avs.as_deref(),
        &chain,
        config,
        Some(wallet),
        Some(password),
    )
    .await?;
    let ivynet_inner = Arc::new(RwLock::new(avs_provider));

    // NOTE: Due to limitations with Prost / GRPC, we create a new server with a reference-counted
    // handle to the inner type for each server, as opposed to cloning / being able to clone the
    // outer service.
    let avs_server = AvsServer::new(IvynetService::new(ivynet_inner.clone()));
    let operator_server =
        OperatorServer::new(IvynetService::new(ivynet_inner.clone()));

    let server =
        Server::new(avs_server, None, None).add_service(operator_server);

    println!("Starting the IvyNet service on port {}...", port);

    server.serve(port).await.expect("Failed to start IvyNet Service.");
    Ok(())
}
