use std::sync::Arc;

use crate::{
    db::{metric::Metric, node::DbNode, node_data::DbNodeData, Account},
    error::BackendError,
    grpc::grpc::messages::SignedDeleteNodeData,
};
use ivynet_core::{
    avs::names::AvsName,
    ethers::types::{Address, Signature},
    grpc::{
        self,
        backend::backend_server::{Backend, BackendServer},
        client::{Request, Response},
        messages::{RegistrationCredentials, SignedLogs, SignedMetrics, SignedNodeData},
        server, Status,
    },
    signature::{recover_delete_node_data, recover_metrics, recover_node_data},
};
use semver::Version;
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
            .attach_node(&self.pool, &node_id, &req.hostname)
            .await
            .map_err(|_| Status::not_found(format!("Cannot register new node for {account:?}",)))?;
        debug!("User {} has registered new node with address {:?}", &req.email, node_id);

        Ok(Response::new(()))
    }

    async fn logs(&self, _request: Request<SignedLogs>) -> Result<Response<()>, Status> {
        // TODO: Implement parsing of the logs
        Ok(Response::new(()))
    }

    async fn metrics(&self, request: Request<SignedMetrics>) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let node_id = recover_metrics(
            &req.metrics,
            &Signature::try_from(req.signature.as_slice())
                .map_err(|_| Status::invalid_argument("Signature is invalid"))?,
        )
        .await?;

        let node = DbNode::get(&self.pool, &node_id)
            .await
            .map_err(|_| Status::not_found("Node not registered"))?;

        _ = Metric::record(
            &self.pool,
            &node,
            &req.metrics.iter().map(|v| v.into()).collect::<Vec<_>>(),
        )
        .await
        .map_err(|_| Status::internal("Failed while saving metrics"))?;

        Ok(Response::new(()))
    }

    async fn node_data(&self, request: Request<SignedNodeData>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        if let Some(node_data) = &req.node_data {
            let node_id = recover_node_data(
                node_data,
                &Signature::try_from(req.signature.as_slice())
                    .map_err(|_| Status::invalid_argument("Signature is invalid"))?,
            )?;

            let _node = DbNode::get(&self.pool, &node_id)
                .await
                .map_err(|_| Status::not_found("Node not registered"))?;

            println!("NODE: {:#?}", _node);

            DbNodeData::record_avs_node_data(
                &self.pool,
                &node_id,
                &AvsName::from(node_data.avs_name.as_str()),
                &Version::parse(&node_data.avs_version)
                    .expect("Cannot parse version on NodeData grpc message"),
                node_data.active_set,
            )
            .await
            .map_err(|e| Status::internal(format!("Failed while saving node_data: {}", e)))?;
        } else {
            return Err(Status::invalid_argument("Node data is missing"));
        }

        Ok(Response::new(()))
    }

    async fn delete_node_data(
        &self,
        request: Request<SignedDeleteNodeData>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let node_id = recover_delete_node_data(
            req.avs_name.clone(),
            &Signature::try_from(req.signature.as_slice())
                .map_err(|_| Status::invalid_argument("Signature is invalid"))?,
        )?;

        DbNodeData::delete_avs_node_data(
            &self.pool,
            &node_id,
            &AvsName::from(req.avs_name.as_str()),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed while deleting node_data: {}", e)))?;

        todo!()
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
