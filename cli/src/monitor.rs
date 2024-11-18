use ivynet_core::{
    grpc::{self, backend::backend_client::BackendClient},
    telemetry::listen,
};
use tracing::info;

pub async fn start_monitor() -> Result<(), anyhow::Error> {
    let config = ivynet_core::config::IvyConfig::load_from_default_path()?;

    let identity_wallet = config.identity_wallet()?;
    let machine_id = config.machine_id;
    let backend_url = config.get_server_url()?;
    let backend_ca = config.get_server_ca();
    let backend_ca = if backend_ca.is_empty() { None } else { Some(backend_ca) };

    let backend_client = BackendClient::new(
        grpc::client::create_channel(grpc::client::Source::Uri(backend_url), backend_ca)
            .await
            .expect("Cannot create channel"),
    );

    info!("Starting monitor listener...");
    listen(backend_client, machine_id, identity_wallet).await?;
    Ok(())
}
