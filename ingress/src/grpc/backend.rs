use crate::error::IngressError;
use db::{
    data::{
        machine_data::build_system_metrics,
        node_data::{update_avs_active_set, update_avs_version},
    },
    log::{ContainerLog, LogLevel},
    metric::Metric,
    Account, Avs, AvsVersionHash, Machine,
};
use ivynet_core::ethers::types::Address;

use ivynet_docker_registry::node_types::get_node_type;
use ivynet_grpc::{
    self,
    backend::backend_server::{Backend, BackendServer},
    client::{Request, Response},
    messages::{
        MachineData, Metrics, NodeData, NodeDataV2, NodeType as NodeTypeMessage, NodeTypeQueries,
        NodeTypes, RegistrationCredentials, SignedLog, SignedMachineData, SignedMetrics,
        SignedNameChange, SignedNodeData, SignedNodeDataV2,
    },
    server, Status,
};

use ivynet_docker::logs::{find_log_level, find_or_create_log_timestamp, sanitize_log};
use ivynet_node_type::NodeType;
use sqlx::PgPool;
use std::{str::FromStr, sync::Arc};
use tracing::debug;
use uuid::Uuid;

use super::data_validator::validate_request;

pub struct BackendService {
    pool: Arc<PgPool>,
}

impl BackendService {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

type NameChange = (String, String); //Old name, new name

#[ivynet_grpc::async_trait]
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

        let (machine_id, log) = validate_request::<String, SignedLog>(
            &self.pool,
            &request.machine_id,
            &request.signature,
            Some(request.log),
        )
        .await?;

        let avs_name = request.avs_name;
        let log = sanitize_log(log.as_str());
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

    async fn machine_data(
        &self,
        request: Request<SignedMachineData>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let (machine_id, machine_data) = validate_request::<MachineData, SignedMachineData>(
            &self.pool,
            &req.machine_id,
            &req.signature,
            req.machine_data,
        )
        .await?;

        let system_metrics = build_system_metrics(&machine_data);

        Machine::update_client_version(&self.pool, &machine_id, &machine_data.ivynet_version)
            .await
            .map_err(|e| Status::internal(format!("Failed while updating client version: {e}")))?;

        _ = Metric::record(
            &self.pool,
            machine_id,
            None,
            &system_metrics.iter().map(|v| v.into()).collect::<Vec<_>>(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed while saving system metrics: {e:?}")))?;

        Ok(Response::new(()))
    }

    async fn node_data(&self, request: Request<SignedNodeData>) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let (machine_id, node_data) = validate_request::<NodeData, SignedNodeData>(
            &self.pool,
            &req.machine_id,
            &req.signature,
            req.node_data,
        )
        .await?;

        let recovered_node_data = RecoveredNodeData::from(node_data);

        process_node_data(&self.pool, machine_id, recovered_node_data).await?;

        Ok(Response::new(()))
    }

    async fn node_data_v2(
        &self,
        request: Request<SignedNodeDataV2>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let (machine_id, node_data) = validate_request::<NodeDataV2, SignedNodeDataV2>(
            &self.pool,
            &req.machine_id,
            &req.signature,
            req.node_data,
        )
        .await?;

        let recovered_node_data = RecoveredNodeData::from(node_data);

        process_node_data(&self.pool, machine_id, recovered_node_data).await?;

        Ok(Response::new(()))
    }

    async fn metrics(&self, request: Request<SignedMetrics>) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let (machine_id, metrics) = validate_request::<Vec<Metrics>, SignedMetrics>(
            &self.pool,
            &req.machine_id,
            &req.signature,
            Some(req.metrics),
        )
        .await?;

        _ = Metric::record(
            &self.pool,
            machine_id,
            req.avs_name.as_deref(),
            &metrics.iter().map(|v| v.into()).collect::<Vec<_>>(),
        )
        .await
        .map_err(|e| Status::internal(format!("Failed while saving metrics: {e:?}")))?;

        Ok(Response::new(()))
    }

    async fn node_type_queries(
        &self,
        request: Request<NodeTypeQueries>,
    ) -> Result<Response<NodeTypes>, Status> {
        let req = request.into_inner();
        let queries = req.node_types;
        let digests: Vec<String> = queries.iter().map(|q| q.image_digest.clone()).collect();

        let potential_hashes = AvsVersionHash::get_versions_from_digests(&self.pool, &digests)
            .await
            .map_err(|e| Status::internal(format!("Failed on database fetch {e}")))?
            .into_iter()
            .map(|(digest, avs_type)| (digest, NodeType::from(avs_type.as_str())))
            .collect();
        let potential_hashes = Some(potential_hashes);

        let node_types = queries
            .into_iter()
            .map(|query| {
                let node_type = get_node_type(
                    &potential_hashes,
                    &query.image_digest,
                    &query.image_name,
                    &query.container_name,
                )
                .unwrap_or(NodeType::Unknown)
                .to_string();
                NodeTypeMessage { container_name: query.container_name, node_type }
            })
            .collect();
        Ok(Response::new(NodeTypes { node_types }))
    }

    async fn name_change(
        &self,
        request: Request<SignedNameChange>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let (machine_id, name_change) = validate_request::<NameChange, SignedNameChange>(
            &self.pool,
            &req.machine_id,
            &req.signature,
            Some((req.old_name, req.new_name)),
        )
        .await?;

        Avs::update_name(&self.pool, machine_id, &name_change.0, &name_change.1)
            .await
            .map_err(|e| Status::internal(format!("Failed while updating machine name: {e}")))?;

        Metric::update_name_on_metrics(&self.pool, machine_id, &name_change.0, &name_change.1)
            .await
            .map_err(|e| {
                Status::internal(format!("Failed while updating machine name on metrics: {e}"))
            })?;

        Ok(Response::new(()))
    }
}

pub async fn serve(
    pool: Arc<PgPool>,
    tls_cert: Option<String>,
    tls_key: Option<String>,
    port: u16,
) -> Result<(), IngressError> {
    tracing::info!("Starting GRPC server on port {port}");
    server::Server::new(BackendServer::new(BackendService::new(pool)), tls_cert, tls_key)
        .serve(server::Endpoint::Port(port))
        .await?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct RecoveredNodeData {
    pub name: String,
    pub node_type: Option<String>,
    pub manifest: Option<String>,
    pub metrics_alive: Option<bool>,
    pub node_running: Option<bool>,
}

impl From<NodeData> for RecoveredNodeData {
    fn from(node_data: NodeData) -> Self {
        RecoveredNodeData {
            name: node_data.name,
            node_type: Some(node_data.node_type),
            manifest: Some(node_data.manifest),
            metrics_alive: Some(node_data.metrics_alive),
            node_running: None,
        }
    }
}

impl From<NodeDataV2> for RecoveredNodeData {
    fn from(node_data: NodeDataV2) -> Self {
        RecoveredNodeData {
            name: node_data.name,
            node_type: node_data.node_type,
            manifest: node_data.manifest,
            metrics_alive: node_data.metrics_alive,
            node_running: node_data.node_running,
        }
    }
}

async fn process_node_data(
    pool: &PgPool,
    machine_id: Uuid,
    node_data: RecoveredNodeData,
) -> Result<(), Status> {
    let name = node_data.name;
    let node_type = node_data.node_type;
    let manifest = node_data.manifest;
    let metrics_alive = node_data.metrics_alive;
    let node_running = node_data.node_running;

    match (node_type, manifest) {
        (Some(node_type), Some(manifest)) => {
            let nt = match NodeType::from(node_type.as_str()) {
                NodeType::Unknown => AvsVersionHash::get_avs_type_from_hash(pool, &manifest)
                    .await
                    .unwrap_or(NodeType::Unknown),
                node_type => node_type,
            };
            Avs::record_avs_data_from_client(pool, machine_id, &name, &nt, &manifest)
                .await
                .map_err(|e| Status::internal(format!("Failed while saving node_data: {e}")))?;
            _ = update_avs_version(pool, machine_id, &name, &manifest).await;
        }
        (None, Some(manifest)) => {
            _ = update_avs_version(pool, machine_id, &name, &manifest).await;
        }
        _ => {}
    }

    if let Some(metrics_alive) = metrics_alive {
        Avs::update_metrics_alive(pool, machine_id, &name, metrics_alive).await.map_err(|e| {
            Status::internal(format!("Failed while setting metrics available flag: {e}"))
        })?;
    }

    if let Some(node_running) = node_running {
        Avs::update_node_running(pool, machine_id, &name, node_running).await.map_err(|e| {
            Status::internal(format!("Failed while setting metrics available flag: {e}"))
        })?;
    }

    _ = update_avs_active_set(pool, machine_id, &name).await;

    Ok(())
}
