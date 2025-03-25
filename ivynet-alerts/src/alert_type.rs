use std::fmt::Display;

use ethers::types::Address;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{EnumCount, EnumIter, EnumProperty, IntoDiscriminant, IntoEnumIterator};
use strum_macros::EnumDiscriminants;
use utoipa::{
    openapi::{RefOr, Schema},
    ToSchema,
};
use uuid::Uuid;

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    EnumCount,
    EnumDiscriminants,
    EnumProperty,
    EnumIter,
    ToSchema,
)]
#[strum_discriminants(name(AlertType))]
#[repr(usize)]
pub enum Alert {
    Custom {
        node_name: String,
        node_type: String,
        extra_data: serde_json::Value,
    } = 1,
    ActiveSetNoDeployment {
        node_name: String,
        node_type: String,
        operator: Address,
    } = 2,
    UnregisteredFromActiveSet {
        node_name: String,
        node_type: String,
        operator: Address,
    } = 3,
    MachineNotResponding {
        machine: Uuid,
    } = 4,
    NodeNotResponding {
        node_name: String,
        node_type: String,
    } = 5,
    NodeNotRunning {
        node_name: String,
        node_type: String,
    } = 6,
    NoChainInfo {
        node_name: String,
        node_type: String,
    } = 7,
    NoMetrics {
        node_name: String,
        node_type: String,
    } = 8,
    NoOperatorId {
        node_name: String,
        node_type: String,
    } = 9,
    HardwareResourceUsage {
        machine: Uuid,
        resource: String,
        percent: u16,
    } = 10,
    // TODO: Find out how exactly this should be used
    LowPerformanceScore {
        node_name: String,
        node_type: String,
        performance: u16,
    } = 11,
    NeedsUpdate {
        node_name: String,
        node_type: String,
        current_version: String,
        recommended_version: String,
    } = 12,
    NewEigenAvs {
        address: Address,
        block_number: u64,
        log_index: u64,
        name: String,
        metadata_uri: String,
        description: String,
        website: String,
        logo: String,
        twitter: String,
    } = 13,
    UpdatedEigenAvs {
        address: Address,
        block_number: u64,
        log_index: u64,
        name: String,
        metadata_uri: String,
        description: String,
        website: String,
        logo: String,
        twitter: String,
    } = 14,
    NoClientHeartbeat,
    NoMachineHeartbeat,
    NoNodeHeartbeat,
}

// Implement ToSchema for AlertType
impl<'s> ToSchema<'s> for AlertType {
    fn schema() -> (&'s str, utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>) {
        (
            "AlertType",
            RefOr::T(Schema::Object(
                utoipa::openapi::schema::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::SchemaType::String)
                    .enum_values(Some(
                        AlertType::list_all()
                            .into_iter()
                            .map(|alert_type| alert_type.to_string())
                            .map(Value::String)
                            .collect::<Vec<Value>>(),
                    ))
                    .description(Some("Type of alert that can be triggered"))
                    .build(),
            )),
        )
    }
}

impl Alert {
    pub fn id(&self) -> usize {
        self.discriminant().id()
    }
    // Generate a UUIDv5 seed for the notification. Uses a combination of stable parameters
    // on the notification type (EG: not time, or percentage, which may vary between
    // notifications, even though they apply to an ongoing condition) and the notification type
    // id to prevent collision where different notification types may have the same interior field.
    pub fn uuid_seed(&self) -> String {
        match self {
            Alert::ActiveSetNoDeployment { node_name, .. } |
            Alert::UnregisteredFromActiveSet { node_name, .. } |
            Alert::NodeNotResponding { node_name, .. } |
            Alert::NodeNotRunning { node_name, .. } |
            Alert::NoChainInfo { node_name, .. } |
            Alert::NoMetrics { node_name, .. } |
            Alert::NoOperatorId { node_name, .. } => {
                format!("{}-{}", node_name, self.id())
            }
            Alert::Custom { .. } => {
                format!("{:?}-{}", self, self.id())
            }
            Alert::MachineNotResponding { machine, .. } => {
                format!("{}-{}", machine, self.id())
            }
            Alert::HardwareResourceUsage { machine, resource, .. } => {
                format!("{}-{}-{}", machine, resource, self.id())
            }
            Alert::LowPerformanceScore { node_name, .. } => {
                format!("{}-{}", node_name, self.id())
            }
            Alert::NeedsUpdate { node_name, current_version, .. } => {
                format!("{}-{}-{}", node_name, current_version, self.id())
            }
            Alert::NewEigenAvs { address, block_number, log_index, .. } => {
                format!("{}-{}-{}", address, block_number, log_index)
            }
            Alert::UpdatedEigenAvs { address, block_number, log_index, .. } => {
                format!("{}-{}-{}", address, block_number, log_index)
            }
            Alert::NoClientHeartbeat => "NoClientHeartbeat".to_string(),
            Alert::NoMachineHeartbeat => "NoMachineHeartbeat".to_string(),
            Alert::NoNodeHeartbeat => "NoNodeHeartbeat".to_string(),
        }
    }

    pub fn variant_count() -> usize {
        Alert::COUNT
    }
}

impl AlertType {
    pub fn id(&self) -> usize {
        self.into()
    }

    pub fn variant_count() -> usize {
        Alert::variant_count()
    }

    pub fn list_all() -> Vec<AlertType> {
        Alert::iter().map(|a| a.into()).collect()
    }
}

impl Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertType::Custom => write!(f, "Custom"),
            AlertType::ActiveSetNoDeployment => write!(f, "ActiveSetNoDeployment"),
            AlertType::UnregisteredFromActiveSet => write!(f, "UnregisteredFromActiveSet"),
            AlertType::MachineNotResponding => write!(f, "MachineNotResponding"),
            AlertType::NodeNotResponding => write!(f, "NodeNotResponding"),
            AlertType::NodeNotRunning => write!(f, "NodeNotRunning"),
            AlertType::NoChainInfo => write!(f, "NoChainInfo"),
            AlertType::NoMetrics => write!(f, "NoMetrics"),
            AlertType::NoOperatorId => write!(f, "NoOperatorId"),
            AlertType::HardwareResourceUsage => write!(f, "HardwareResourceUsage"),
            AlertType::LowPerformanceScore => write!(f, "LowPerformanceScore"),
            AlertType::NeedsUpdate => write!(f, "NeedsUpdate"),
            AlertType::NewEigenAvs => write!(f, "NewEigenAvs"),
            AlertType::UpdatedEigenAvs => write!(f, "UpdatedEigenAvs"),
            AlertType::NoClientHeartbeat => write!(f, "NoClientHeartbeat"),
            AlertType::NoMachineHeartbeat => write!(f, "NoMachineHeartbeat"),
            AlertType::NoNodeHeartbeat => write!(f, "NoNodeHeartbeat"),
        }
    }
}

impl Serialize for AlertType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for AlertType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Custom" => Ok(AlertType::Custom),
            "ActiveSetNoDeployment" => Ok(AlertType::ActiveSetNoDeployment),
            "UnregisteredFromActiveSet" => Ok(AlertType::UnregisteredFromActiveSet),
            "MachineNotResponding" => Ok(AlertType::MachineNotResponding),
            "NodeNotResponding" => Ok(AlertType::NodeNotResponding),
            "NodeNotRunning" => Ok(AlertType::NodeNotRunning),
            "NoChainInfo" => Ok(AlertType::NoChainInfo),
            "NoMetrics" => Ok(AlertType::NoMetrics),
            "NoOperatorId" => Ok(AlertType::NoOperatorId),
            "HardwareResourceUsage" => Ok(AlertType::HardwareResourceUsage),
            "LowPerformanceScore" => Ok(AlertType::LowPerformanceScore),
            "NeedsUpdate" => Ok(AlertType::NeedsUpdate),
            "NewEigenAvs" => Ok(AlertType::NewEigenAvs),
            "UpdatedEigenAvs" => Ok(AlertType::UpdatedEigenAvs),
            "NoClientHeartbeat" => Ok(AlertType::NoClientHeartbeat),
            "NoMachineHeartbeat" => Ok(AlertType::NoMachineHeartbeat),
            "NoNodeHeartbeat" => Ok(AlertType::NoNodeHeartbeat),
            _ => Err(serde::de::Error::custom("Unknown alert type")),
        }
    }
}

// This implementation MUST be exhaustive. This and the reverse should probably be a macro.
impl From<&AlertType> for usize {
    fn from(alert_type: &AlertType) -> usize {
        match alert_type {
            AlertType::Custom => 1,
            AlertType::ActiveSetNoDeployment => 2,
            AlertType::UnregisteredFromActiveSet => 3,
            AlertType::MachineNotResponding => 4,
            AlertType::NodeNotResponding => 5,
            AlertType::NodeNotRunning => 6,
            AlertType::NoChainInfo => 7,
            AlertType::NoMetrics => 8,
            AlertType::NoOperatorId => 9,
            AlertType::HardwareResourceUsage => 10,
            AlertType::LowPerformanceScore => 11,
            AlertType::NeedsUpdate => 12,
            AlertType::NewEigenAvs => 13,
            AlertType::UpdatedEigenAvs => 14,
            AlertType::NoClientHeartbeat => 15,
            AlertType::NoMachineHeartbeat => 16,
            AlertType::NoNodeHeartbeat => 17,
        }
    }
}

impl From<usize> for AlertType {
    fn from(id: usize) -> Self {
        match id {
            1 => AlertType::Custom,
            2 => AlertType::ActiveSetNoDeployment,
            3 => AlertType::UnregisteredFromActiveSet,
            4 => AlertType::MachineNotResponding,
            5 => AlertType::NodeNotResponding,
            6 => AlertType::NodeNotRunning,
            7 => AlertType::NoChainInfo,
            8 => AlertType::NoMetrics,
            9 => AlertType::NoOperatorId,
            10 => AlertType::HardwareResourceUsage,
            11 => AlertType::LowPerformanceScore,
            12 => AlertType::NeedsUpdate,
            13 => AlertType::NewEigenAvs,
            14 => AlertType::UpdatedEigenAvs,
            15 => AlertType::NoClientHeartbeat,
            16 => AlertType::NoMachineHeartbeat,
            17 => AlertType::NoNodeHeartbeat,
            _ => panic!("Unknown alert type"),
        }
    }
}
