use std::sync::Arc;

use crate::error::IngressError;
use ivynet_database::{
    alerts::{
        alert_db::AlertDb,
        node::alert_handler::{alerts_from_avs, resolve_node_alerts},
        node::alerts_active::NewNodeAlert,
        org::alert_handler::OrganizationAlertHandler,
    },
    eigen_avs_metadata::{EigenAvsMetadata, MetadataContent},
    Avs, AvsActiveSet, DbAvsVersionData,
};
use ivynet_error::ethers::types::Address;
use ivynet_grpc::{
    self,
    backend_events::{
        backend_events_server::{BackendEvents, BackendEventsServer},
        LatestBlock, LatestBlockRequest, MetadataUriEvent, RegistrationEvent,
    },
    client::{Request, Response},
    server, Status,
};
use ivynet_notifications::{NotificationConfig, NotificationDispatcher};
use serde_json;
use sqlx::PgPool;

pub struct EventsService {
    pool: PgPool,
    organization_alert_handler: OrganizationAlertHandler,
}

impl EventsService {
    pub fn new(pool: PgPool, organization_alert_handler: OrganizationAlertHandler) -> Self {
        Self { pool, organization_alert_handler }
    }
}

#[ivynet_grpc::async_trait]
impl BackendEvents for EventsService {
    async fn get_latest_block(
        &self,
        request: Request<LatestBlockRequest>,
    ) -> Result<Response<LatestBlock>, Status> {
        let req = request.into_inner();
        let block_number = AvsActiveSet::get_latest_block(&self.pool, &req.address, req.chain_id)
            .await
            .map_err(|a| Status::invalid_argument(format!("Bad arguments provided {a:?}")))?;
        Ok(Response::new(LatestBlock { block_number }))
    }

    async fn report_registration_event(
        &self,
        request: Request<RegistrationEvent>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        AvsActiveSet::record_registration_event(&self.pool, &req)
            .await
            .map_err(|a| Status::invalid_argument(format!("Bad arguments provided {a:?}")))?;

        // Resolve alert step
        let operator_address = Address::from_slice(&req.address);

        let nodes = Avs::get_by_operator_address(&self.pool, &operator_address).await?;

        let version_map = DbAvsVersionData::get_all_avs_version(&self.pool).await?;

        let new_alerts = nodes
            .iter()
            .flat_map(|node| {
                let alerts = alerts_from_avs(node, &version_map);

                alerts
                    .into_iter()
                    .map(|alert| NewNodeAlert::new(node.machine_id, alert, node.avs_name.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        resolve_node_alerts(&self.pool, new_alerts, nodes).await?;

        Ok(Response::new(()))
    }

    async fn report_metadata_uri_event(
        &self,
        request: Request<MetadataUriEvent>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let avs_address = Address::from_slice(&req.avs);
        let metadata_uri = req.metadata_uri;
        let block_number = req.block_number;
        let log_index = req.log_index;
        tracing::debug!("Received metadata uri event: {:#?}", metadata_uri.clone());
        tracing::debug!("Address: {:#?}", avs_address);
        tracing::debug!("Block number: {:#?}", block_number);
        tracing::debug!("Log index: {:#?}", log_index);

        // Use reqwest to get the metadata content
        let metadata = reqwest::get(metadata_uri.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to fetch metadata: {}", e)))?;
        let metadata_text = metadata
            .text()
            .await
            .map_err(|e| Status::internal(format!("Failed to parse metadata content: {}", e)))?;

        // Parse the JSON metadata for cleaner output
        let parsed_metadata: serde_json::Value = serde_json::from_str(&metadata_text)
            .map_err(|e| Status::internal(format!("Failed to parse JSON metadata: {}", e)))?;

        let metadata_content = MetadataContent {
            name: parsed_metadata["name"].as_str().map(|s| s.to_string()),
            description: parsed_metadata["description"].as_str().map(|s| s.to_string()),
            website: parsed_metadata["website"].as_str().map(|s| s.to_string()),
            logo: parsed_metadata["logo"].as_str().map(|s| s.to_string()),
            twitter: parsed_metadata["twitter"].as_str().map(|s| s.to_string()),
        };

        //Needs to be above the insert because count checks for dupes
        self.organization_alert_handler
            .handle_new_eigen_avs_alerts(
                &self.pool,
                &avs_address,
                block_number,
                log_index,
                &metadata_uri,
                &metadata_content,
            )
            .await
            .map_err(|e| {
                Status::internal(format!("Failed to handle new eigen avs alerts: {}", e))
            })?;

        EigenAvsMetadata::insert(
            &self.pool,
            avs_address,
            block_number as i64,
            req.log_index as i32,
            metadata_uri.clone(),
            metadata_content.clone(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed to insert metadata: {}", e)))?;

        Ok(Response::new(()))
    }
}

pub async fn serve(
    pool: PgPool,
    notification_config: NotificationConfig,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    port: u16,
) -> Result<(), IngressError> {
    tracing::info!("Starting GRPC events server on port {port}");

    let notification_dispatcher =
        Arc::new(NotificationDispatcher::new(notification_config, AlertDb::new(pool.clone())));

    server::Server::new(
        BackendEventsServer::new(EventsService::new(
            pool.clone(),
            OrganizationAlertHandler::new(notification_dispatcher.clone(), pool),
        )),
        tls_cert,
        tls_key,
    )
    .serve(server::Endpoint::Port(port))
    .await?;

    Ok(())
}
