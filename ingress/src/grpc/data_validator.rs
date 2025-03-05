use ivynet_database::machine::Machine;
use ivynet_error::ethers::types::{Signature, H160};

use ivynet_grpc::{
    self,
    messages::{
        MachineData, Metrics, NodeData, NodeDataV2, SignedClientLog, SignedLog, SignedMachineData,
        SignedMetrics, SignedNameChange, SignedNodeData, SignedNodeDataV2,
    },
    Status,
};
use ivynet_signer::sign_utils::{
    recover_client_log, recover_log, recover_machine_data, recover_metrics, recover_name_change,
    recover_node_data, recover_node_data_v2,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct SignedData<T> {
    pub machine_id: Uuid,
    pub client_id: H160,
    pub data: T,
}

pub trait SignedDataValidator {
    type DataType;

    fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> impl std::future::Future<Output = Result<H160, Status>> + Send;
}

// Common validation logic
pub async fn validate_request<T, V>(
    pool: &PgPool,
    machine_id: &[u8],
    signature: &[u8],
    data: Option<T>,
) -> Result<SignedData<T>, Status>
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

    Ok(SignedData { machine_id, client_id, data })
}

// Implementation for v1
impl SignedDataValidator for SignedNodeData {
    type DataType = NodeData;

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status> {
        recover_node_data(data, signature).map_err(|e| {
            Status::invalid_argument(format!("Failed to recover signature for node data: {e}"))
        })
    }
}

// Implementation for v2
impl SignedDataValidator for SignedNodeDataV2 {
    type DataType = NodeDataV2;

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status> {
        recover_node_data_v2(data, signature).map_err(|e| {
            Status::invalid_argument(format!("Failed to recover signature for node data v2: {e}"))
        })
    }
}

impl SignedDataValidator for SignedMetrics {
    type DataType = Vec<Metrics>;

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status> {
        recover_metrics(data, signature).map_err(|e| {
            Status::invalid_argument(format!("Failed to recover signature for metrics: {e}"))
        })
    }
}

impl SignedDataValidator for SignedNameChange {
    type DataType = (String, String); //old name, new name

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status> {
        recover_name_change(data.0.as_str(), data.1.as_str(), signature).map_err(|e| {
            Status::invalid_argument(format!("Failed to recover signature for name change: {e}"))
        })
    }
}

impl SignedDataValidator for SignedLog {
    type DataType = String;

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status> {
        recover_log(data, signature).map_err(|e| {
            Status::invalid_argument(format!("Failed to recover signature for logs: {e}"))
        })
    }
}

impl SignedDataValidator for SignedClientLog {
    type DataType = String;

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status> {
        recover_client_log(data, signature).map_err(|e| {
            Status::invalid_argument(format!("Failed to recover signature for client logs: {e}"))
        })
    }
}

impl SignedDataValidator for SignedMachineData {
    type DataType = MachineData;

    async fn recover_signature(
        data: &Self::DataType,
        signature: &Signature,
    ) -> Result<H160, Status> {
        recover_machine_data(data, signature).map_err(|e| {
            Status::invalid_argument(format!("Failed to recover signature for machine data: {e}"))
        })
    }
}
