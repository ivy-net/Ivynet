use crate::error::IngressError;
use db::{
    data::node_data::{update_avs_active_set, update_avs_version},
    log::{ContainerLog, LogLevel},
    machine::Machine,
    metric::Metric,
    Account, Avs, AvsVersionHash,
};
use ivynet_core::ethers::types::{Address, Signature, H160};

use ivynet_docker_registry::node_types::get_node_type;
use ivynet_grpc::{
    self,
    backend::backend_server::{Backend, BackendServer},
    client::{Request, Response},
    messages::{
        Metrics, NodeType as NodeTypeMessage, NodeTypeQueries, NodeTypes, RegistrationCredentials,
        SignedLog, SignedMetrics, SignedNameChange,
    },
    node_data::{NodeData, NodeDataV2, SignedNodeData, SignedNodeDataV2},
    server, Status,
};

use ivynet_docker::logs::{find_log_level, find_or_create_log_timestamp, sanitize_log};
use ivynet_node_type::NodeType;
use ivynet_signer::sign_utils::{
    recover_from_string, recover_metrics, recover_name_change, recover_node_data,
    recover_node_data_v2,
};
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

#[derive(Debug, Clone)]
pub struct RecoveredNodeData {
    signature: Vec<u8>,
    machine_id: Vec<u8>,
    name: String,
    node_type: Option<String>,
    manifest: Option<String>,
    metrics_alive: Option<bool>,
    node_running: Option<bool>,
    ivynet_version: Option<String>,
}

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

        let (machine_id, node_data) = validate_request::<NodeData, SignedNodeData>(
            &self.pool,
            &req.machine_id,
            &req.signature,
            req.node_data,
        )
        .await?;

        let NodeData { name, node_type, manifest, metrics_alive } = node_data;

        let nt = match NodeType::from(node_type.as_str()) {
            NodeType::Unknown => AvsVersionHash::get_avs_type_from_hash(&self.pool, &manifest)
                .await
                .unwrap_or(NodeType::Unknown),
            node_type => node_type,
        };

        Avs::record_avs_data_from_client(&self.pool, machine_id, &name, &nt, &manifest)
            .await
            .map_err(|e| Status::internal(format!("Failed while saving node_data: {e}")))?;

        Avs::update_metrics_alive(&self.pool, machine_id, &name, metrics_alive).await.map_err(
            |e| Status::internal(format!("Failed while setting metrics available flag: {e}")),
        )?;

        _ = update_avs_version(&self.pool, machine_id, &name, &manifest).await;
        _ = update_avs_active_set(&self.pool, machine_id, &name).await;

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

        let NodeDataV2 { name, node_type, manifest, metrics_alive, node_running } =
            node_data.clone();

        debug!("Node data: {:#?}", node_data);

        match (node_type, manifest) {
            (Some(node_type), Some(manifest)) => {
                let nt = match NodeType::from(node_type.as_str()) {
                    NodeType::Unknown => {
                        AvsVersionHash::get_avs_type_from_hash(&self.pool, &manifest)
                            .await
                            .unwrap_or(NodeType::Unknown)
                    }
                    node_type => node_type,
                };
                Avs::record_avs_data_from_client(&self.pool, machine_id, &name, &nt, &manifest)
                    .await
                    .map_err(|e| Status::internal(format!("Failed while saving node_data: {e}")))?;
                _ = update_avs_version(&self.pool, machine_id, &name, &manifest).await;
            }
            (None, Some(manifest)) => {
                _ = update_avs_version(&self.pool, machine_id, &name, &manifest).await;
            }
            _ => {}
        }

        if let Some(metrics_alive) = metrics_alive {
            Avs::update_metrics_alive(&self.pool, machine_id, &name, metrics_alive).await.map_err(
                |e| Status::internal(format!("Failed while setting metrics available flag: {e}")),
            )?;
        }

        if let Some(node_running) = node_running {
            Avs::update_node_running(&self.pool, machine_id, &name, node_running).await.map_err(
                |e| Status::internal(format!("Failed while setting metrics available flag: {e}")),
            )?;
        }
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

        let client_id = recover_name_change(
            &req.old_name,
            &req.new_name,
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

        Avs::update_name(&self.pool, machine_id, &req.old_name, &req.new_name)
            .await
            .map_err(|e| Status::internal(format!("Failed while updating machine name: {e}")))?;

        Metric::update_name_on_metrics(&self.pool, machine_id, &req.old_name, &req.new_name)
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

trait SignedDataValidator {
    type DataType;

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status>;
}

// Common validation logic
async fn validate_request<T, V>(
    pool: &PgPool,
    machine_id: &[u8],
    signature: &[u8],
    data: Option<T>,
) -> Result<(Uuid, T), Status>
where
    V: SignedDataValidator<DataType = T>,
{
    // Only relevant for node_data
    // Handle the Option<NodeData> case
    let data = if let Some(data) = data {
        data
    } else {
        return Err(Status::invalid_argument("Data missing from payload".to_string()));
    };

    // Validate signature
    let signature = Signature::try_from(signature)
        .map_err(|_| Status::invalid_argument("Signature is invalid"))?;

    let client_id = V::recover_signature(&data, &signature).await?;

    // Validate machine ID
    let machine_id = Uuid::from_slice(machine_id)
        .map_err(|e| Status::invalid_argument(format!("Machine id has wrong length ({e:?})")))?;

    // Check machine ownership
    if !Machine::is_owned_by(pool, &client_id, machine_id).await.unwrap_or(false) {
        return Err(Status::not_found("Machine not registered for given client".to_string()));
    }

    Ok((machine_id, data))
}

// Implementation for v1
impl SignedDataValidator for SignedNodeData {
    type DataType = NodeData;

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status> {
        recover_node_data(data, signature)
            .await
            .map_err(|e| Status::invalid_argument(format!("Failed to recover signature: {e}")))
    }
}

// Implementation for v2
impl SignedDataValidator for SignedNodeDataV2 {
    type DataType = NodeDataV2;

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status> {
        recover_node_data_v2(data, signature)
            .await
            .map_err(|e| Status::invalid_argument(format!("Failed to recover signature: {e}")))
    }
}

// impl SignedDataValidator for SignedMetrics {
//     type DataType = Vec<Metric>;

//     async fn recover_signature(
//         data: &Self::DataType,
//         signature: &Signature,
//     ) -> Result<H160, Status> {
//         recover_metrics(data, signature)
//             .await
//             .map_err(|e| Status::invalid_argument(format!("Failed to recover signature: {e}")))
//     }
// }
