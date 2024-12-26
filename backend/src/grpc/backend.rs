use crate::{
    data::node_data::{update_avs_active_set, update_avs_version},
    db::{
        log::{ContainerLog, LogLevel},
        machine::Machine,
        metric::Metric,
        Account, Avs, AvsVersionHash,
    },
    error::BackendError,
};
use ivynet_core::{
    ethers::types::{Address, Signature},
    grpc::{
        self,
        backend::backend_server::{Backend, BackendServer},
        client::{Request, Response},
        messages::{
            Digests, NodeData, NodeType as NodeTypeMessage, NodeTypes, RegistrationCredentials,
            SignedLog, SignedMetrics, SignedNodeData,
        },
        server, Status,
    },
    signature::{recover_from_string, recover_metrics, recover_node_data},
};

use ivynet_docker::logs::{find_log_level, find_or_create_log_timestamp, sanitize_log};
use ivynet_node_type::NodeType;
use sqlx::PgPool;
use std::{str::FromStr, sync::Arc};
use tracing::debug;
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

        let machine_id = Uuid::from_slice(&request.machine_id)
            .map_err(|_| Status::invalid_argument("Machine id has wrong length".to_string()))?;
        let avs_name = request.avs_name;
        let log = sanitize_log(&request.log);
        let log_level = LogLevel::from_str(&find_log_level(&log))
            .map_err(|_| Status::invalid_argument("Log level is invalid".to_string()))?;
        let created_at = Some(find_or_create_log_timestamp(&log));
        let log =
            ContainerLog { machine_id, avs_name, log, log_level, created_at, other_fields: None };
        debug!("STORING LOG: {:?}", log);

        ContainerLog::record(&self.pool, &log)
            .await
            .map_err(|e| Status::internal(format!("Failed while saving logs: {e:?}")))?;

        Ok(Response::new(()))
    }

    async fn node_data(&self, request: Request<SignedNodeData>) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        // TODO: Why does it force this to be an option even though it's not in the proto?
        let node_data = if let Some(node_data) = req.node_data {
            node_data
        } else {
            return Err(Status::invalid_argument("Node data is missing".to_string()));
        };

        let client_id = recover_node_data(
            &node_data,
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

        let NodeData { name, node_type, manifest, metrics_alive } = node_data;

        let avs_type = match NodeType::from(node_type.as_str()) {
            NodeType::Unknown => AvsVersionHash::get_avs_type_from_hash(&self.pool, &manifest)
                .await
                .unwrap_or(NodeType::Unknown),
            node_type => node_type,
        };

        Avs::record_avs_data_from_client(&self.pool, machine_id, &name, &avs_type, &manifest)
            .await
            .map_err(|e| Status::internal(format!("Failed while saving node_data: {e}")))?;

        Avs::update_metrics_alive(&self.pool, machine_id, &name, metrics_alive).await.map_err(
            |e| Status::internal(format!("Failed while setting metrics available flag: {e}")),
        )?;

        _ = update_avs_version(&self.pool, machine_id, &name, &manifest).await;
        _ = update_avs_active_set(&self.pool, machine_id, &name).await;

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

    async fn node_types(&self, request: Request<Digests>) -> Result<Response<NodeTypes>, Status> {
        let req = request.into_inner();
        let types = AvsVersionHash::get_versions_from_digests(&self.pool, &req.digests)
            .await
            .map_err(|e| Status::internal(format!("Failed on database fetch {e}")))?;
        Ok(Response::new(NodeTypes {
            node_types: types
                .into_iter()
                .map(|nt| (NodeTypeMessage { digest: nt.0, node_type: nt.1 }))
                .collect::<Vec<_>>(),
        }))
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
