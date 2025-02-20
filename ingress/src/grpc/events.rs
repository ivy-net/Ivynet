use crate::error::IngressError;
use ivynet_database::AvsActiveSet;
use ivynet_grpc::{
    self,
    backend_events::{
        backend_events_server::{BackendEvents, BackendEventsServer},
        LatestBlock, LatestBlockRequest, MetadataUriEvent, RegistrationEvent,
    },
    client::{Request, Response},
    server, Status,
};
use sqlx::PgPool;
use std::sync::Arc;

pub struct EventsService {
    pool: Arc<PgPool>,
}

impl EventsService {
    pub fn new(pool: Arc<PgPool>) -> Self {
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
        _request: Request<MetadataUriEvent>,
    ) -> Result<Response<()>, Status> {
        // info!("Need to implement db side of this");
        Ok(Response::new(()))
    }
}

pub async fn serve(
    pool: Arc<PgPool>,
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
