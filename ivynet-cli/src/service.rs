use clap::Subcommand;
use dialoguer::Password;
use ivynet_core::{
    config::IvyConfig,
    error::IvyError,
    grpc::{ivynet_api::ivy_daemon_avs::avs_server::AvsServer, server::Server},
    server::build_avs_provider,
    wallet::IvyWallet,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{api::avs::AvsService, error::Error};

const DEFAULT_PORT: u16 = 55501;

pub async fn serve(
    avs: Option<String>,
    chain: Option<String>,
    port: Option<u16>,
    config: &IvyConfig,
) -> Result<(), Error> {
    let port = port.unwrap_or(DEFAULT_PORT);

    // Keystore load
    let password: String = Password::new().with_prompt("Input the password for your stored keyfile").interact()?;
    let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), password)?;

    // Avs Service
    // TODO: This should default to local instead of holesky?
    let chain = chain.unwrap_or_else(|| "holesky".to_string());
    let avs_provider = build_avs_provider(avs.as_deref(), &chain, config, Some(wallet)).await?;
    let avs_service = AvsService::new(Arc::new(RwLock::new(avs_provider)));
    let avs_server = AvsServer::new(avs_service);

    let server = Server::new(avs_server, None, None);

    println!("Starting the IvyNet service on port {}...", port);

    server.serve(port).await.expect("Failed to start IvyNet Service.");
    Ok(())
}
