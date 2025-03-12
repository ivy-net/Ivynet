use ivynet_grpc::messages::{MachineData, Metrics, MetricsAttribute};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    avs::Avs,
    data::node_data::{build_avs_info, AvsInfo},
    error::DatabaseError,
    machine::Machine,
    metric::Metric,
};

const UPTIME_METRIC: &str = "uptime";
const CORES_METRIC: &str = "cores";
const CPU_USAGE_METRIC: &str = "cpu_usage";
const MEMORY_USAGE_METRIC: &str = "ram_usage";
const MEMORY_FREE_METRIC: &str = "free_ram";
const MEMORY_TOTAL_METRIC: &str = "memory_total";
const DISK_USAGE_METRIC: &str = "disk_usage";
const DISK_FREE_METRIC: &str = "free_disk";
const DISK_TOTAL_METRIC: &str = "disk_total";
const DISK_INFO_METRIC: &str = "disk_info";
const DISK_ID_METRIC: &str = "disk_id";

#[derive(Serialize, ToSchema, Clone, Debug, Default)]
pub struct MachineStatusReport {
    pub total_machines: usize,
    pub healthy_machines: Vec<String>,
    pub unhealthy_machines: Vec<String>,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub enum MachineStatus {
    Healthy,
    Unhealthy,
}

#[derive(Serialize, Debug, Clone)]
pub enum MachineError {
    Idle,
    SystemResourcesUsage,
    ClientUpdateRequired,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct MachineInfoReport {
    pub machine_id: String,
    pub name: String,
    pub status: MachineStatus,
    pub client_version: Option<String>,
    pub hardware_info: HardwareUsageInfo,
    pub errors: Vec<MachineError>,
    pub avs_list: Vec<AvsInfo>,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct HardwareUsageInfo {
    pub sys_metrics: SystemMetrics,
    pub memory_status: HardwareInfoStatus,
    pub disk_status: HardwareInfoStatus,
}

#[derive(Serialize, ToSchema, Clone, Debug, PartialEq, Eq)]
pub enum HardwareInfoStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct SystemMetrics {
    pub cpu_cores: u64,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub memory_free: u64,
    pub memory_total: u64,
    pub disks: Vec<DiskInfo>,
    pub uptime: u64,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct DiskInfo {
    pub id: String,
    pub total: u64,
    pub free: u64,
    pub used: u64,
}

pub async fn build_machine_info(
    pool: &sqlx::PgPool,
    machine: &Machine,
    machine_metrics: HashMap<String, Metric>,
) -> Result<MachineInfoReport, DatabaseError> {
    let mut errors = vec![];

    let hardware_info = build_system_metrics(&machine_metrics);

    if hardware_info.disk_status == HardwareInfoStatus::Critical
        || hardware_info.memory_status == HardwareInfoStatus::Critical
    {
        errors.push(MachineError::SystemResourcesUsage);
    }

    let avses = Avs::get_machines_avs_list(pool, machine.machine_id).await?;
    let mut avs_infos = vec![];

    if avses.is_empty() {
        errors.push(MachineError::Idle);
    }

    for avs in avses {
        let metrics =
            Metric::get_organized_for_avs(pool, machine.machine_id, &avs.avs_name.to_string())
                .await?;
        let avs_info = build_avs_info(pool, avs.clone(), metrics).await?;

        avs_infos.push(avs_info);
    }

    if machine.client_version.is_none() {
        errors.push(MachineError::ClientUpdateRequired);
    }

    let info_report = MachineInfoReport {
        machine_id: format!("{:?}", machine.machine_id),
        name: format!("{:?}", machine.name),
        client_version: machine.client_version.clone(),
        status: if errors.is_empty() { MachineStatus::Healthy } else { MachineStatus::Unhealthy },
        hardware_info,
        avs_list: avs_infos,
        errors,
    };

    Ok(info_report)
}

pub async fn get_machine_health(
    pool: &PgPool,
    machine_ids: Vec<Uuid>,
) -> Result<(Vec<Uuid>, Vec<Uuid>), DatabaseError> {
    let mut unhealthy_list: Vec<Uuid> = vec![];
    let mut healthy_list: Vec<Uuid> = vec![];

    for machine_id in machine_ids {
        let Some(machine) = Machine::get(pool, machine_id).await? else {
            continue;
        };

        let machine_metrics = Metric::get_machine_metrics_only(pool, machine_id).await?;
        let machine_info = build_machine_info(pool, &machine, machine_metrics).await?;

        if machine_info.errors.is_empty() {
            healthy_list.push(machine_id);
        } else {
            unhealthy_list.push(machine_id);
        }
    }

    Ok((healthy_list, unhealthy_list))
}

pub fn build_system_metrics(machine_metrics: &HashMap<String, Metric>) -> HardwareUsageInfo {
    let mut disks = Vec::new();

    // Collect disk information from disk_info_X metrics
    let mut i = 0;
    while let Some(disk_metric) = machine_metrics.get(&format!("{}_{}", DISK_INFO_METRIC, i)) {
        if let Some(attrs) = &disk_metric.attributes {
            let disk_info = DiskInfo {
                id: attrs.get(DISK_ID_METRIC).cloned().unwrap_or_default(),
                total: attrs
                    .get(DISK_TOTAL_METRIC)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default(),
                free: attrs.get(DISK_FREE_METRIC).and_then(|v| v.parse().ok()).unwrap_or_default(),
                used: attrs.get(DISK_USAGE_METRIC).and_then(|v| v.parse().ok()).unwrap_or_default(),
            };
            disks.push(disk_info);
        }
        i += 1;
    }

    let metrics = SystemMetrics {
        cpu_cores: machine_metrics.get(CORES_METRIC).map(|m| m.value as u64).unwrap_or_default(),
        cpu_usage: machine_metrics.get(CPU_USAGE_METRIC).map(|m| m.value).unwrap_or_default(),
        memory_usage: machine_metrics
            .get(MEMORY_USAGE_METRIC)
            .map(|m| m.value as u64)
            .unwrap_or_default(),
        memory_free: machine_metrics
            .get(MEMORY_FREE_METRIC)
            .map(|m| m.value as u64)
            .unwrap_or_default(),
        memory_total: machine_metrics
            .get(MEMORY_TOTAL_METRIC)
            .map(|m| m.value as u64)
            .unwrap_or_default(),
        disks,
        uptime: machine_metrics.get(UPTIME_METRIC).map(|m| m.value as u64).unwrap_or_default(),
    };

    // Calculate memory status
    let memory_status = if metrics.memory_usage == 0 && metrics.memory_free == 0 {
        HardwareInfoStatus::Healthy
    } else {
        let total = metrics.memory_usage + metrics.memory_free;
        if total == 0 {
            HardwareInfoStatus::Healthy
        } else if metrics.memory_usage as f64 > (total as f64 * 0.95) {
            HardwareInfoStatus::Critical
        } else if metrics.memory_usage as f64 > (total as f64 * 0.9) {
            HardwareInfoStatus::Warning
        } else {
            HardwareInfoStatus::Healthy
        }
    };

    // Calculate disk status
    let disk_status = if metrics.disks.is_empty() {
        HardwareInfoStatus::Healthy
    } else {
        let mut worst_status = HardwareInfoStatus::Healthy;
        for disk in &metrics.disks {
            let total = disk.used + disk.free;
            if total == 0 {
                continue;
            }
            if disk.used as f64 > (total as f64 * 0.95) {
                worst_status = HardwareInfoStatus::Critical;
                break;
            } else if disk.used as f64 > (total as f64 * 0.9) {
                worst_status = HardwareInfoStatus::Warning;
            }
        }
        worst_status
    };

    HardwareUsageInfo { sys_metrics: metrics, memory_status, disk_status }
}

pub fn convert_system_metrics(sys_info: &MachineData) -> Vec<Metrics> {
    let mut sys_metrics = vec![
        Metrics {
            name: UPTIME_METRIC.to_owned(),
            value: sys_info.uptime.parse::<f64>().unwrap(),
            attributes: Default::default(),
        },
        Metrics {
            name: CPU_USAGE_METRIC.to_owned(),
            value: sys_info.cpu_usage.parse::<f64>().unwrap(),
            attributes: Default::default(),
        },
        Metrics {
            name: CORES_METRIC.to_owned(),
            value: sys_info.cpu_cores.parse::<f64>().unwrap(),
            attributes: Default::default(),
        },
        Metrics {
            name: MEMORY_USAGE_METRIC.to_owned(),
            value: sys_info.memory_used.parse::<f64>().unwrap(),
            attributes: Default::default(),
        },
        Metrics {
            name: MEMORY_FREE_METRIC.to_owned(),
            value: sys_info.memory_free.parse::<f64>().unwrap(),
            attributes: Default::default(),
        },
        Metrics {
            name: MEMORY_TOTAL_METRIC.to_owned(),
            value: sys_info.memory_total.parse::<f64>().unwrap(),
            attributes: Default::default(),
        },
        Metrics {
            name: DISK_TOTAL_METRIC.to_owned(),
            value: sys_info.disk_used_total.parse::<f64>().unwrap(),
            attributes: Default::default(),
        },
    ];
    for (i, disk) in sys_info.disks.iter().enumerate() {
        let disk_attributes = vec![
            MetricsAttribute { name: DISK_ID_METRIC.to_owned(), value: disk.id.to_string() },
            MetricsAttribute { name: DISK_USAGE_METRIC.to_owned(), value: disk.used.to_string() },
            MetricsAttribute { name: DISK_FREE_METRIC.to_owned(), value: disk.free.to_string() },
            MetricsAttribute { name: DISK_TOTAL_METRIC.to_owned(), value: disk.total.to_string() },
        ];
        sys_metrics.push(Metrics {
            name: format!("{}_{}", DISK_INFO_METRIC, i),
            value: 0.0,
            attributes: disk_attributes,
        });
    }

    sys_metrics
}
