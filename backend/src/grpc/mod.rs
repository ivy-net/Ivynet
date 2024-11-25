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
        messages::{RegistrationCredentials, SignedLog, SignedMetrics, SignedNodeData},
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

    async fn logs(&self, request: Request<SignedLog>) -> Result<Response<()>, Status> {
        let request = request.into_inner();
        debug!("Received logs: {:?}", request.log);

        let client_id = recover_from_string(
            &request.log,
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
            serde_json::from_str::<Vec<ContainerLog>>(&request.log).map_err(|e| {
                error!("{:?} || Logs: {:?}", request.log, e);
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
        let machine_id = Uuid::from_slice(&req.machine_id).map_err(|e| {
            Status::invalid_argument(format!("Machine id has wrong length ({e:?})"))
        })?;

        if !Machine::is_owned_by(&self.pool, &client_id, machine_id).await.unwrap_or(false) {
            return Err(Status::not_found("Machine not registered for given client".to_string()));
        }

        _ = Metric::record(
            &self.pool,
            machine_id,
            req.avs_name.as_deref(),
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

            let version = node_data
                .avs_version
                .as_deref()
                .and_then(|v| Version::parse(v).ok())
                .unwrap_or_else(|| Version::new(0, 0, 0));

            if !node_data.avs_name.is_empty() {
                Avs::record_avs_data_from_client(
                    &self.pool,
                    machine_id,
                    &node_data.avs_name,
                    &NodeType::from(node_data.avs_type.as_str()),
                    &version,
                )
                .await
                .map_err(|e| Status::internal(format!("Failed while saving node_data: {e}")))?;
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
        let _req = request.into_inner();

        // if let Some(node_data) = &req.node_data {
        //     let client_id = recover_node_data(
        //         node_data,
        //         &Signature::try_from(req.signature.as_slice())
        //             .map_err(|_| Status::invalid_argument("Signature is invalid"))?,
        //     )?;

        //     let machine_id = Uuid::from_slice(&node_data.machine_id)
        //         .map_err(|_| Status::invalid_argument("Machine id has wrong
        // length".to_string()))?;

        //     if !Machine::is_owned_by(&self.pool, &client_id, machine_id).await.unwrap_or(false) {
        //         return Err(Status::not_found(
        //             "Machine not registered for given client".to_string(),
        //         ));
        //     }
        //     Avs::delete_avs_data(
        //         &self.pool,
        //         machine_id,
        //         &Address::from_slice(&node_data.operator_id),
        //         node_data.avs_name.as_str(),
        //         &NodeType::try_from(node_data.avs_name.as_str())
        //             .map_err(|_| Status::invalid_argument("Bad AVS name provided"))?,
        //     )
        //     .await
        //     .map_err(|e| Status::internal(format!("Failed while deleting node_data: {}", e)))?;
        //     Ok(Response::new(()))
        // } else {
        //     Err(Status::invalid_argument("Node data is missing"))
        // }
        Ok(Response::new(()))
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
