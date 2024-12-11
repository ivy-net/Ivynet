use std::str::FromStr as _;

use clap::Parser;
use ethers::types::Address;
use ivynet_core::grpc::{
    backend_events::backend_events_client::BackendEventsClient,
    client::{create_channel, Uri},
};
use scraper::blockchain;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::FmtSubscriber;

#[derive(Clone, Parser)]
pub struct Params {
    #[arg(long, env = "BACKEND_URL", value_parser = Uri::from_str, default_value = if cfg!(debug_assertions) {
        "http://localhost:50051"
    } else {
        "https://api2.test.ivynet.dev:50051"
    })]
    pub backend_uri: Uri,

    #[arg(long, env = "GRPC_TLS_CA")]
    pub grpc_tls_ca: Option<String>,

    #[arg(long, env = "RPC_URL")]
    pub rpc_url: String,

    #[arg(long, env = "START_BLOCK", default_value_t = 0)]
    pub start_block: u64,

    #[arg(long, env = "ADDRESSES")]
    pub addresses: Option<String>,

    #[arg(long, env = "LOG_LEVEL", default_value_t = LevelFilter::INFO)]
    pub log_level: LevelFilter,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Now it's time to load all configured data
    let params = Params::parse();

    start_tracing(params.log_level)?;
    let addresses = params
        .addresses
        .map(|addr| addr.split(",").filter_map(|a| a.parse::<Address>().ok()).collect::<Vec<_>>())
        .unwrap_or_else(Vec::new);

    info!("IvyNet scrapper service starting...");

    let backend = BackendEventsClient::new(
        create_channel(
            ivynet_core::grpc::client::Source::Uri(params.backend_uri),
            params.grpc_tls_ca,
        )
        .await?,
    );

    blockchain::fetch(&params.rpc_url, backend, params.start_block, &addresses).await?;

    Ok(())
}

fn start_tracing(level: LevelFilter) -> Result<(), anyhow::Error> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
