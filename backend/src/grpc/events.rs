use ivynet_core::grpc::{
    self,
    backend_events::{
        backend_events_server::{BackendEvents, BackendEventsServer},
        Event, LatestBlock, LatestBlockRequest,
    },
    client::{Request, Response},
    server, Status,
};
use sqlx::PgPool;
use std::sync::Arc;

use crate::{db::AvsActiveSet, error::BackendError};

pub struct EventsService {
    pool: Arc<PgPool>,
}

impl EventsService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[grpc::async_trait]
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

    async fn report_event(&self, request: Request<Event>) -> Result<Response<()>, Status> {
        AvsActiveSet::record_event(&self.pool, &request.into_inner())
            .await
            .map_err(|a| Status::invalid_argument(format!("Bad arguments provided {a:?}")))?;
        Ok(Response::new(()))
    }
}

pub async fn serve(
    pool: Arc<PgPool>,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    port: u16,
) -> Result<(), BackendError> {
    tracing::info!("Starting GRPC events server on port {port}");
    server::Server::new(BackendEventsServer::new(EventsService::new(pool)), tls_cert, tls_key)
        .serve(server::Endpoint::Port(port))
        .await?;

    Ok(())
}
