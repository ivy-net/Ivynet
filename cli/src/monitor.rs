pub async fn start_monitor() -> Result<(), anyhow::Error> {
    let config = ivynet_core::config::IvyConfig::load_from_default_path()?;
    let identity_wallet = config.identity_wallet()?;
    // Spawn telemetry listener
    Ok(())
}
