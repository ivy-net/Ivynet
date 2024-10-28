use crate::{error::Error, rpc::ivynet::IvynetService, telemetry};
use ivynet_core::{
    avs::{
        build_avs_provider, eigenda::EigenDA, lagrange::Lagrange, mach_avs::AltLayer,
        names::AvsName, AvsVariant,
    },
    config::IvyConfig,
    docker::dockercmd::DockerCmd,
    fluentd::log_server::serve_log_server,
    grpc::{
        backend::backend_client::BackendClient,
        client::{create_channel, Uri},
        ivynet_api::{
            ivy_daemon_avs::avs_server::AvsServer,
            ivy_daemon_operator::operator_server::OperatorServer,
        },
        server::{Endpoint, Server},
    },
    messenger::BackendMessenger,
    rpc_management::connect_provider,
};
use std::sync::Arc;
use tokio::{signal::ctrl_c, sync::RwLock};
use tracing::{error, info};

pub async fn serve(
    avs: Option<String>,
    chain: Option<String>,
    config: &IvyConfig,
    server_url: Uri,
    server_ca: Option<String>,
    no_backend: bool,
) -> Result<(), Error> {
    let sock = Endpoint::Path(config.uds_dir());

    let backend_client = BackendClient::new(
        create_channel(ivynet_core::grpc::client::Source::Uri(server_url), server_ca).await?,
    );
    let messenger = if no_backend {
        None
    } else {
        Some(BackendMessenger::new(backend_client.clone(), config.identity_wallet()?))
    };

    // Avs Service

    // TODO: This should default to local instead of holesky?
    let chain = chain.unwrap_or_else(|| "holesky".to_string());
    let mut avs_provider =
        build_avs_provider(avs.as_deref(), &chain, config, None, None, messenger).await?;
    if let Some(configured_service) = &config.configured_service {
        let avs_instance: Box<dyn AvsVariant> = match configured_service.service {
            AvsName::EigenDA => Box::new(EigenDA::new_from_chain(configured_service.chain)),
            AvsName::AltLayer => Box::new(AltLayer::new_from_chain(configured_service.chain)),
            AvsName::LagrangeZK => Box::new(Lagrange::new_from_chain(configured_service.chain)),
            _ => panic!("Unsupported AVS configured"),
        };

        if avs_provider
            .set_avs(
                avs_instance,
                connect_provider(&config.get_rpc_url(configured_service.chain)?, None)
                    .await?
                    .into(),
            )
            .await
            .is_ok()
        {
            info!(
                "Configured network {:?} with AVS {:?}",
                configured_service.chain, configured_service.service
            );

            match configured_service.autostart {
                ivynet_core::config::StartMode::No => {}
                ivynet_core::config::StartMode::Yes => {
                    if avs_provider.start().await.is_ok() {
                        info!("Configured AVS started!");
                    } else {
                        error!("Unable to start the AVS");
                    }
                }
                ivynet_core::config::StartMode::Attach => {
                    if avs_provider.attach().await.is_ok() {
                        info!("Configured AVS attached!");
                    } else {
                        error!("Unable to attach configured AVS");
                    }
                }
            }
        } else {
            error!("Unable to select AVS {:?}", configured_service.service);
        }
    }

    let ivynet_inner = Arc::new(RwLock::new(avs_provider));

    ///////////////////
    // Logging
    ///////////////////

    // Set logging directory
    let fluentd_path = config.get_dir().join("fluentd");
    std::env::set_var("FLUENTD_PATH", fluentd_path.to_str().unwrap());
    info!("Serving local logs at {:?}", fluentd_path);
    // Start the container
    let _fluentd = DockerCmd::new()
        .args(["up", "-d", "--build", "--force-recreate"])
        .current_dir(&fluentd_path)
        .spawn_dockerchild()
        .await?;
    info!("Fluentd logging container started");

    ///////////////////
    // GRPC
    ///////////////////

    // NOTE: Due to limitations with Prost / GRPC, we create a new server with a
    // reference-counted handle to the inner type for each server, as opposed to cloning
    // / being able to clone the outer service.
    let avs_server = AvsServer::new(IvynetService::new(ivynet_inner.clone(), config));
    let operator_server = OperatorServer::new(IvynetService::new(ivynet_inner.clone(), config));

    let server = Server::new(avs_server, None, None).add_service(operator_server);
    info!("Starting server...");

    if no_backend {
        tokio::select! {
            ret = server.serve(sock) => { error!("Local server error {ret:?}") },
            _= ctrl_c() => {
                info!("Shutting down")
            }
        }
    } else {
        let connection_wallet = config.identity_wallet()?;
        tokio::select! {
            ret = server.serve(sock) => { error!("Local server error {ret:?}") },
            ret = serve_log_server(backend_client.clone(), connection_wallet.clone()) => { error!("Log server error {ret:?}") }
            ret = telemetry::listen(ivynet_inner, backend_client, connection_wallet) => { error!("Telemetry listener error {ret:?}") }
            _= ctrl_c() => {
                info!("Shutting down")
            }
        }
    }
    Ok(())
}
