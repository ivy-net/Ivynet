use std::str::FromStr;

use clap::Parser;
use ivynet_core::grpc::client::Uri;
use tracing::Level;

mod version_hash {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

#[derive(Clone, Parser, Debug)]
#[command(name = "ivynet-ingress", version = version_hash::VERSION_HASH, about = "The IvyNet backend system")]
pub struct Config {
    #[arg(long, env = "IVY_OTLP_URL", value_parser = Uri::from_str)]
    pub otlp_url: Option<Uri>,

    #[arg(long, env = "IVY_LOG_LEVEL", default_value_t = Level::INFO)]
    pub log_level: Level,

    #[arg(long, env = "IVY_GRPC_TLS_CA")]
    pub grpc_tls_ca: Option<String>,

    #[arg(long, env = "IVY_GRPC_TLS_CERT")]
    pub grpc_tls_cert: Option<String>,

    #[arg(long, env = "IVY_GRPC_TLS_KEY")]
    pub grpc_tls_key: Option<String>,

    #[arg(long, env = "IVY_GRPC_PORT", default_value_t = 50050)]
    pub grpc_port: u16,

    #[arg(long, env = "IVY_EVENTS_TLS_CA")]
    pub events_tls_ca: Option<String>,

    #[arg(long, env = "IVY_EVENTS_TLS_CERT")]
    pub events_tls_cert: Option<String>,

    #[arg(long, env = "IVY_EVENTS_TLS_KEY")]
    pub events_tls_key: Option<String>,

    #[arg(long, env = "IVY_EVENTS_PORT", default_value_t = 50051)]
    pub events_port: u16,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgresql://ivy:secret_ivy@localhost:5432/ivynet"
    )]
    pub db_uri: String,
}
