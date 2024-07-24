use std::sync::Arc;

use crate::{db::Account, error::BackendError};
use ivynet_core::{
    ethers::types::Address,
    grpc::{
        self,
        backend::backend_server::{Backend, BackendServer},
        client::{Request, Response},
        messages::{RegistrationCredentials, SignedMetrics},
        server, Status,
    },
};
use sqlx::PgPool;
use tracing::debug;

pub struct BackendService {
    pool: Arc<PgPool>,
}

impl BackendService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[grpc::async_trait]
impl Backend for BackendService {
    async fn register(
        &self,
        request: Request<RegistrationCredentials>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let account =
            Account::verify(&self.pool, &req.email, &req.password).await.map_err(|_| {
                Status::not_found(format!("User {} not found or password is incorrect", req.email))
            })?;
        let node_id = Address::from_slice(&req.public_key);
        account
            .attach_node(&self.pool, &node_id)
            .await
            .map_err(|_| Status::not_found(format!("Cannot register new node for {account:?}",)))?;
        debug!("User {} has registered new node with address {:?}", &req.email, node_id);

        Ok(Response::new(()))
    }

    async fn metrics(&self, _request: Request<SignedMetrics>) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("Not implemented yet"))
    }
}

pub async fn serve(
    pool: Arc<PgPool>,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    port: u16,
) -> Result<(), BackendError> {
    tracing::info!("Starting GRPC server on port {port}");
    server::Server::new(BackendServer::new(BackendService::new(pool)), tls_cert, tls_key)
        .serve(server::Endpoint::Port(port))
        .await?;

    Ok(())
}
