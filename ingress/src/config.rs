use std::str::FromStr;

use clap::Parser;
use ivynet_grpc::client::Uri;
use ivynet_notifications::{NotificationConfig, SendgridSpecificTemplates, SendgridTemplates};
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

    #[arg(long, env = "IVY_ALERTS_TLS_CA")]
    pub alerts_tls_ca: Option<String>,

    #[arg(long, env = "IVY_ALERTS_TLS_CERT")]
    pub alerts_tls_cert: Option<String>,

    #[arg(long, env = "IVY_ALERTS_TLS_KEY")]
    pub alerts_tls_key: Option<String>,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgresql://ivy:secret_ivy@localhost:5432/ivynet"
    )]
    pub db_uri: String,

    #[arg(long, env = "SENDGRID_KEY")]
    pub sendgrid_key: Option<String>,

    #[arg(long, env = "SENDGRID_FROM")]
    pub sendgrid_from: Option<String>,

    #[arg(long, env = "STN_GENERIC")]
    pub stn_generic: Option<String>,

    #[arg(long, env = "STN_CUSTOM")]
    pub stn_custom: Option<String>,

    #[arg(long, env = "STN_UNREG_ACTIVE_SET")]
    pub stn_unreg_active_set: Option<String>,

    #[arg(long, env = "STN_MACHINE_NOT_RESPONDING")]
    pub stn_machine_not_responding: Option<String>,

    #[arg(long, env = "STN_NODE_NOT_RUNNING")]
    pub stn_node_not_running: Option<String>,

    #[arg(long, env = "STN_NO_CHAIN_INFO")]
    pub stn_no_chain_info: Option<String>,

    #[arg(long, env = "STN_NO_METRICS")]
    pub stn_no_metrics: Option<String>,

    #[arg(long, env = "STN_NO_OPERATOR")]
    pub stn_no_operator: Option<String>,

    #[arg(long, env = "STN_HW_RES_USAGE")]
    pub stn_hw_res_usage: Option<String>,

    #[arg(long, env = "STN_LOW_PERFORMANCE")]
    pub stn_low_performance: Option<String>,

    #[arg(long, env = "STN_NEEDS_UPDATE")]
    pub stn_needs_update: Option<String>,

    #[arg(long, env = "TELEGRAM_TOKEN")]
    pub telegram_token: Option<String>,
}

impl From<Config> for NotificationConfig {
    fn from(val: Config) -> Self {
        NotificationConfig {
            telegram_token: val.telegram_token.unwrap_or_default(),
            sendgrid_key: val.sendgrid_key.unwrap_or_default(),
            sendgrid_from: val.sendgrid_from.unwrap_or_default(),
            sendgrid_templates: if let Some(generic) = val.stn_generic {
                SendgridTemplates::Generic(generic)
            } else {
                SendgridTemplates::Specific(Box::new(SendgridSpecificTemplates {
                    custom: val.stn_custom.unwrap_or_default(),
                    unreg_active_set: val.stn_unreg_active_set.unwrap_or_default(),
                    machine_not_responding: val.stn_machine_not_responding.unwrap_or_default(),
                    node_not_running: val.stn_node_not_running.unwrap_or_default(),
                    no_chain_info: val.stn_no_chain_info.unwrap_or_default(),
                    no_metrics: val.stn_no_metrics.unwrap_or_default(),
                    no_operator: val.stn_no_operator.unwrap_or_default(),
                    hw_res_usage: val.stn_hw_res_usage.unwrap_or_default(),
                    low_perf: val.stn_low_performance.unwrap_or_default(),
                    needs_update: val.stn_needs_update.unwrap_or_default(),
                }))
            },
        }
    }
}
