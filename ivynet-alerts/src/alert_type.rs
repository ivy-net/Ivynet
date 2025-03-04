use std::fmt::Display;

use ethers::types::Address;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter, IntoDiscriminant, IntoEnumIterator};
use strum_macros::EnumDiscriminants;
use uuid::Uuid;

use crate::alert_contents::Node;

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, EnumCount, EnumDiscriminants, EnumIter,
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
    MachineNotResponding = 4,
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
    LowPerformaceScore {
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
            Alert::Custom { node_name, .. } |
            Alert::ActiveSetNoDeployment { node_name, .. } |
            Alert::UnregisteredFromActiveSet { node_name, .. } |
            Alert::NodeNotResponding { node_name, .. } |
            Alert::NodeNotRunning { node_name, .. } |
            Alert::NoChainInfo { node_name, .. } |
            Alert::NoMetrics { node_name, .. } |
            Alert::NoOperatorId { node_name, .. } => {
                format!("{}-{}", node_name, self.id())
            }
            Alert::MachineNotResponding => {
                format!("{:?}-{}", self, self.id())
            }
            Alert::HardwareResourceUsage { machine, resource, .. } => {
                format!("{}-{}-{}", machine, resource, self.id())
            }
            Alert::LowPerformaceScore { node_name, .. } => {
                format!("{}-{}", node_name, self.id())
            }
            Alert::NeedsUpdate { node_name, current_version, .. } => {
                format!("{}-{}-{}", node_name, current_version, self.id())
            }
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
            AlertType::LowPerformaceScore => write!(f, "LowPerformaceScore"),
            AlertType::NeedsUpdate => write!(f, "NeedsUpdate"),
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
            "LowPerformaceScore" => Ok(AlertType::LowPerformaceScore),
            "NeedsUpdate" => Ok(AlertType::NeedsUpdate),
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
            AlertType::LowPerformaceScore => 11,
            AlertType::NeedsUpdate => 12,
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
            11 => AlertType::LowPerformaceScore,
            12 => AlertType::NeedsUpdate,
            _ => panic!("Unknown alert type"),
        }
    }
}
