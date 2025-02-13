use crate::error::IngressError;
use ivynet_grpc::{
    self,
    alerts::{
        alerts_server::{Alerts, AlertsServer},
        SignedAcknowledgeAlert, SignedAlert, SignedResolveAlert,
    },
    client::{Request, Response},
    server, Status,
};
use sqlx::PgPool;
use std::sync::Arc;

pub struct AlertService {
    #[allow(dead_code)]
    pool: Arc<PgPool>,
}

impl AlertService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[ivynet_grpc::async_trait]
impl Alerts for AlertService {
    async fn new_alert(&self, _request: Request<SignedAlert>) -> Result<Response<()>, Status> {
        todo!("New alerts unimplemented")
    }

    async fn acknowledge_alert(
        &self,
        _request: Request<SignedAcknowledgeAlert>,
    ) -> Result<Response<()>, Status> {
        todo!("Acknowledge alerts unimplemented")
    }

    async fn resolve_alert(
        &self,
        _request: Request<SignedResolveAlert>,
    ) -> Result<Response<()>, Status> {
        todo!("Resolve alerts unimplemented")
    }
}

pub async fn serve(
    pool: Arc<PgPool>,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    port: u16,
) -> Result<(), IngressError> {
    tracing::info!("Starting GRPC events server on port {port}");
    server::Server::new(AlertsServer::new(AlertService::new(pool)), tls_cert, tls_key)
        .serve(server::Endpoint::Port(port))
        .await?;

    Ok(())
}
