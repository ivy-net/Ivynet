use std::sync::Arc;

use clap::Parser as _;
use db::configure;
use ivynet_ingress::{config::Config, error::IngressError, grpc};
use ivynet_notifications::{NotificationConfig, SendgridTemplates};
use tracing::{error, Level};
use tracing_subscriber::FmtSubscriber;

pub fn start_tracing(level: Level) -> Result<(), IngressError> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), IngressError> {
    let config = Config::parse();
    start_tracing(config.log_level)?;
    let pool = Arc::new(configure(&config.db_uri, false).await?);

    let notification_config = NotificationConfig {
        telegram_token: &config.telegram_token.unwrap_or_else(|| "".to_string()),
        sendgrid_key: &config.sendgrid_key.unwrap_or_else(|| "".to_string()),
        sendgrid_from: &config.sendgrid_from.unwrap_or_else(|| "".to_string()),
        sendgrid_templates: SendgridTemplates {
            custom: &config.stn_custom.unwrap_or_else(|| "".to_string()),
            unreg_active_set: &config.stn_unreg_active_set.unwrap_or_else(|| "".to_string()),
            machine_not_responding: &config
                .stn_machine_not_responding
                .unwrap_or_else(|| "".to_string()),
            node_not_running: &config.stn_node_not_running.unwrap_or_else(|| "".to_string()),
            no_chain_info: &config.stn_no_chain_info.unwrap_or_else(|| "".to_string()),
            no_metrics: &config.stn_no_metrics.unwrap_or_else(|| "".to_string()),
            no_operator: &config.stn_no_operator.unwrap_or_else(|| "".to_string()),
            hw_res_usage: &config.stn_hw_res_usage.unwrap_or_else(|| "".to_string()),
            low_perf: &config.stn_low_performance.unwrap_or_else(|| "".to_string()),
            needs_update: &config.stn_needs_update.unwrap_or_else(|| "".to_string()),
        },
    };

    let grpc_service = grpc::backend_serve(
        pool.clone(),
        notification_config,
        config.grpc_tls_cert,
        config.grpc_tls_key,
        config.grpc_port,
    );

    let events_service = grpc::events_serve(
        pool.clone(),
        config.events_tls_cert,
        config.events_tls_key,
        config.events_port,
    );

    let alerts_service =
        grpc::alerts_serve(pool, config.alerts_tls_cert, config.alerts_tls_key, config.alerts_port);

    tokio::select! {
        e = grpc_service => error!("Executor has stopped. Reason: {e:?}"),
        e = events_service => error!("Events service has stopped. Reason: {e:?}"),
        e = alerts_service => error!("Alerts service has stopped. Reason: {e:?}"),
    }
    Ok(())
}
