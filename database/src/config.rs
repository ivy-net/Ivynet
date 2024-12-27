use clap::Parser;
use tracing::Level;

mod version_hash {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

#[derive(Clone, Parser, Debug)]
#[command(name = "ivynet-database", version = version_hash::VERSION_HASH, about = "The IvyNet database system")]
pub struct Config {
    #[arg(long, env = "GRPC_TLS_CA")]
    pub grpc_tls_ca: Option<String>,

    #[arg(long, env = "GRPC_TLS_CERT")]
    pub grpc_tls_cert: Option<String>,

    #[arg(long, env = "GRPC_TLS_KEY")]
    pub grpc_tls_key: Option<String>,

    #[arg(long, env = "GRPC_PORT", default_value_t = 50070)]
    pub grpc_port: u16,

    #[arg(long, env = "LOG_LEVEL", default_value_t = Level::INFO)]
    pub log_level: Level,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgresql://ivy:secret_ivy@localhost:5432/ivynet"
    )]
    pub db_uri: String,

    #[arg(long, env = "DB_MIGRATE", default_value_t = false)]
    pub migrate: bool,

    #[arg(long)]
    pub add_organization: Option<String>,

    #[arg(long)]
    pub set_avs_version: Option<String>,

    #[arg(long)]
    pub add_avs_version_hash: Option<String>,

    #[arg(long)]
    pub set_breaking_change_version: Option<String>,

    #[arg(long)]
    pub add_node_version_hashes: bool,

    #[arg(long)]
    pub update_node_data_versions: bool,
}
