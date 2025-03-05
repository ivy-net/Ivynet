use std::str::FromStr as _;

use clap::Parser;
use ethers::types::Address;
use ivynet_grpc::{
    backend_events::backend_events_client::BackendEventsClient,
    client::{create_channel, Uri},
};
use scanner::blockchain;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::FmtSubscriber;

mod version_hash {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

#[derive(Clone, Parser, Debug)]
#[command(name = "scraper", version = version_hash::VERSION_HASH, about = "Ivynet scraper service")]
pub struct Params {
    #[arg(long, env = "BACKEND_URL", value_parser = Uri::from_str, default_value = if cfg!(debug_assertions) {
        "http://localhost:50051"
    } else {
        "https://api1.test.ivynet.dev:50051"
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

    #[arg(long, short, default_value_t = false)]
    pub reset_blockheight: bool,
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

    info!("IvyNet scraper service starting...");

    let backend =
        BackendEventsClient::new(create_channel(params.backend_uri, params.grpc_tls_ca).await?);

    blockchain::fetch(
        &params.rpc_url,
        backend,
        params.start_block,
        &addresses,
        params.reset_blockheight,
    )
    .await?;

    Ok(())
}

fn start_tracing(level: LevelFilter) -> Result<(), anyhow::Error> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
