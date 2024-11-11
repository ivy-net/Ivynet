use std::str::FromStr;

use clap::Parser;
use ivynet_core::grpc::client::Uri;
use tracing::Level;

mod version_hash {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

#[derive(Clone, Parser, Debug)]
#[command(name = "ivynet-backend", version = version_hash::VERSION_HASH, about = "The IvyNet backend system")]
pub struct Config {
    #[arg(long, env = "IVY_HTTP_PORT", default_value_t = 8080)]
    pub http_port: u16,

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

    #[arg(long, env = "IVY_ROOT_URL", value_parser = Uri::from_str, default_value = "http://localhost:8080")]
    pub root_url: Uri,

    #[arg(long, env = "SENDGRID_FROM", default_value = "no-reply@em739.ivynet.dev")]
    pub sendgrid_from: Option<String>,

    #[arg(long, env = "SENDGRID_API_KEY")]
    pub sendgrid_api_key: Option<String>,

    #[arg(long, env = "SENDGRID_ORG_VER_TMP")]
    pub org_verification_template: Option<String>,

    #[arg(long, env = "SENDGRID_USER_VER_TMP")]
    pub user_verification_template: Option<String>,

    #[arg(long, env = "SENDGRID_PASS_RESET_TMP")]
    pub pass_reset_template: Option<String>,

    #[arg(long, env = "IVY_CACHE_URL", value_parser = Uri::from_str, default_value = "memcache://localhost:11211" )]
    pub cache_url: Uri,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgresql://ivy:secret_ivy@localhost:5432/ivynet"
    )]
    pub db_uri: String,

    #[arg(long, env = "IVY_MIGRATE", default_value_t = false)]
    pub migrate: bool,

    #[arg(long)]
    pub add_organization: Option<String>,

    #[arg(long)]
    pub set_avs_version: Option<String>,

    #[arg(long)]
    pub set_breaking_change_version: Option<String>,
}
