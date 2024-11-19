use crate::{
    db::{avs::Avs, log::ContainerLog, machine::Machine, metric::Metric, Account},
    error::BackendError,
};
use ivynet_core::{
    ethers::types::{Address, Signature},
    grpc::{
        self,
        backend::backend_server::{Backend, BackendServer},
        client::{Request, Response},
        messages::{RegistrationCredentials, SignedLogs, SignedMetrics, SignedNodeData},
        server, Status,
    },
    node_type::NodeType,
    signature::{recover_from_string, recover_metrics, recover_node_data},
};
use semver::Version;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;
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
        let client_id = Address::from_slice(&req.public_key);
        account
            .attach_client(
                &self.pool,
                &client_id,
                Uuid::from_slice(&req.machine_id)
                    .map_err(|_| Status::invalid_argument("Wrong machine_id size".to_string()))?,
                &req.hostname,
            )
            .await
            .map_err(|_| Status::not_found(format!("Cannot register new node for {account:?}",)))?;
        debug!(
            "User {} has registered new client with address {:?} and machine id {:?}",
            &req.email, client_id, req.machine_id
        );

        Ok(Response::new(()))
    }

    async fn logs(&self, request: Request<SignedLogs>) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        debug!("Received logs: {:?}", request.logs);

        let client_id = recover_from_string(
            &request.logs,
            &Signature::try_from(request.signature.as_slice())
                .map_err(|_| Status::invalid_argument("Signature is invalid"))?,
        )?;

        if !Machine::is_owned_by(
            &self.pool,
            &client_id,
            Uuid::from_slice(&request.machine_id)
                .map_err(|_| Status::invalid_argument("Machine id has wrong length".to_string()))?,
        )
        .await
        .unwrap_or(false)
        {
            return Err(Status::not_found("Machine not registered for given client".to_string()));
        }

        let mut parsed_logs =
            serde_json::from_str::<Vec<ContainerLog>>(&request.logs).map_err(|e| {
                error!("{:?} || Logs: {:?}", request.logs, e);
                Status::invalid_argument(format!("Log deserialization error: {:?}", e))
            })?;

        // TODO: We can also batch insert logs in the future.

        let futures = parsed_logs.iter_mut().map(|log| ContainerLog::record(&self.pool, log));

        let results = futures::future::join_all(futures).await;

        for result in results {
            if let Err(e) = result {
                error!("Failed to save log: {:?}", e);
                return Err(Status::internal("Failed to save log"));
            }
        }

        Ok(Response::new(()))
    }

    async fn metrics(&self, request: Request<SignedMetrics>) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let client_id = recover_metrics(
            &req.metrics,
            &Signature::try_from(req.signature.as_slice())
                .map_err(|_| Status::invalid_argument("Signature is invalid"))?,
        )
        .await?;
        let machine_id = Uuid::from_slice(&req.machine_id)
            .map_err(|_| Status::invalid_argument("Machine id has wrong length".to_string()))?;

        if !Machine::is_owned_by(&self.pool, &client_id, machine_id).await.unwrap_or(false) {
            return Err(Status::not_found("Machine not registered for given client".to_string()));
        }

        _ = Metric::record(
            &self.pool,
            machine_id,
            &req.avs_name,
            &req.metrics.iter().map(|v| v.into()).collect::<Vec<_>>(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed while saving metrics: {e:?}")))?;

        Ok(Response::new(()))
    }

    async fn update_node_data(
        &self,
        request: Request<SignedNodeData>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        if let Some(node_data) = &req.node_data {
            let client_id = recover_node_data(
                node_data,
                &Signature::try_from(req.signature.as_slice())
                    .map_err(|_| Status::invalid_argument("Signature is invalid"))?,
            )?;

            let machine_id = Uuid::from_slice(&node_data.machine_id)
                .map_err(|_| Status::invalid_argument("Machine id has wrong length".to_string()))?;

            if !Machine::is_owned_by(&self.pool, &client_id, machine_id).await.unwrap_or(false) {
                return Err(Status::not_found(
                    "Machine not registered for given client".to_string(),
                ));
            }

            if !node_data.avs_name.is_empty() {
                Avs::record_avs_data(
                    &self.pool,
                    &Address::from_slice(&node_data.operator_id),
                    machine_id,
                    &NodeType::try_from(node_data.avs_name.as_str())
                        .map_err(|_| Status::invalid_argument("Bad AVS name provided"))?,
                    &node_data
                        .avs_version
                        .as_ref()
                        .map(|v| Version::parse(v).unwrap_or(Version::new(0, 0, 0)))
                        .unwrap_or(Version::new(0, 0, 0)),
                    node_data.active_set.unwrap_or(false),
                )
                .await
                .map_err(|e| Status::internal(format!("Failed while saving node_data: {}", e)))?;
            }
            Ok(Response::new(()))
        } else {
            Err(Status::invalid_argument("Node data is missing"))
        }
    }

    async fn delete_node_data(
        &self,
        request: Request<SignedNodeData>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        if let Some(node_data) = &req.node_data {
            let client_id = recover_node_data(
                node_data,
                &Signature::try_from(req.signature.as_slice())
                    .map_err(|_| Status::invalid_argument("Signature is invalid"))?,
            )?;

            let machine_id = Uuid::from_slice(&node_data.machine_id)
                .map_err(|_| Status::invalid_argument("Machine id has wrong length".to_string()))?;

            if !Machine::is_owned_by(&self.pool, &client_id, machine_id).await.unwrap_or(false) {
                return Err(Status::not_found(
                    "Machine not registered for given client".to_string(),
                ));
            }
            Avs::delete_avs_data(
                &self.pool,
                machine_id,
                &Address::from_slice(&node_data.operator_id),
                &NodeType::try_from(node_data.avs_name.as_str())
                    .map_err(|_| Status::invalid_argument("Bad AVS name provided"))?,
            )
            .await
            .map_err(|e| Status::internal(format!("Failed while deleting node_data: {}", e)))?;
            Ok(Response::new(()))
        } else {
            Err(Status::invalid_argument("Node data is missing"))
        }
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
