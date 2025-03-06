use crate::error::IngressError;
use ivynet_database::{
    eigen_avs_metadata::{self, EigenAvsMetadata, MetadataContent},
    AvsActiveSet,
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
use serde_json;
use sqlx::PgPool;

pub struct EventsService {
    pool: PgPool,
}

impl EventsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        AvsActiveSet::record_registration_event(&self.pool, &request.into_inner())
            .await
            .map_err(|a| Status::invalid_argument(format!("Bad arguments provided {a:?}")))?;
        Ok(Response::new(()))
    }

    async fn report_metadata_uri_event(
        &self,
        request: Request<MetadataUriEvent>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let avs = Address::from_slice(&req.avs);
        let metadata_uri = req.metadata_uri;
        let block_number = req.block_number;

        println!("Received metadata uri event: {:#?}", metadata_uri.clone());
        println!("Address: {:#?}", avs);
        println!("Block number: {:#?}", block_number);
        println!("Log index: {:#?}", req.log_index);

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

        println!("Metadata content (parsed): {:#?}", parsed_metadata);

        let metadata_content = MetadataContent {
            name: parsed_metadata["name"].as_str().map(|s| s.to_string()),
            description: parsed_metadata["description"].as_str().map(|s| s.to_string()),
            website: parsed_metadata["website"].as_str().map(|s| s.to_string()),
            logo: parsed_metadata["logo"].as_str().map(|s| s.to_string()),
            twitter: parsed_metadata["twitter"].as_str().map(|s| s.to_string()),
        };

        EigenAvsMetadata::insert(
            &self.pool,
            avs,
            block_number as i64,
            req.log_index as i32,
            metadata_uri,
            metadata_content,
        )
        .await
        .map_err(|e| Status::internal(format!("Failed to insert metadata: {}", e)))?;

        Ok(Response::new(()))
    }
}

pub async fn serve(
    pool: PgPool,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    port: u16,
) -> Result<(), IngressError> {
    tracing::info!("Starting GRPC events server on port {port}");
    server::Server::new(BackendEventsServer::new(EventsService::new(pool)), tls_cert, tls_key)
        .serve(server::Endpoint::Port(port))
        .await?;

    Ok(())
}
