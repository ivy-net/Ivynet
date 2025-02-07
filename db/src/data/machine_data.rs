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
const MEMORY_USAGE_METRIC: &str = "memory_usage";
const MEMORY_FREE_METRIC: &str = "memory_free";
const MEMORY_TOTAL_METRIC: &str = "memory_total";
const DISK_USAGE_METRIC: &str = "disk_usage";
const DISK_FREE_METRIC: &str = "disk_free";
const DISK_TOTAL_METRIC: &str = "disk_total";
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
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct MachineInfoReport {
    pub machine_id: String,
    pub name: String,
    pub status: MachineStatus,
    pub client_version: Option<String>,
    pub system_metrics: SystemMetrics,
    pub avs_list: Vec<AvsInfo>,
    pub errors: Vec<MachineError>,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct HardwareUsageInfo {
    pub usage: f64,
    pub free: f64,
    pub status: HardwareInfoStatus,
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
    pub disk_free: Vec<u64>,
    pub disk_usage: Vec<u64>,
    pub disk_total: u64,
    pub uptime: u64,
}

pub async fn build_machine_info(
    pool: &sqlx::PgPool,
    machine: &Machine,
    machine_metrics: HashMap<String, Metric>,
) -> Result<MachineInfoReport, DatabaseError> {
    let mut errors = vec![];

    let system_metrics = build_system_metrics(&machine_metrics);

    let memory_info = build_hardware_info(
        machine_metrics.get(MEMORY_USAGE_METRIC).cloned(),
        machine_metrics.get(MEMORY_FREE_METRIC).cloned(),
    );

    let disk_info = build_hardware_info(
        machine_metrics.get(DISK_USAGE_METRIC).cloned(),
        machine_metrics.get(DISK_FREE_METRIC).cloned(),
    );

    if disk_info.status == HardwareInfoStatus::Critical ||
        memory_info.status == HardwareInfoStatus::Critical
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

    let info_report = MachineInfoReport {
        machine_id: format!("{:?}", machine.machine_id),
        name: format!("{:?}", machine.name),
        client_version: machine.client_version.clone(),
        status: if errors.is_empty() { MachineStatus::Healthy } else { MachineStatus::Unhealthy },
        system_metrics,
        avs_list: avs_infos,
        errors,
    };

    Ok(info_report)
}

pub fn build_hardware_info(
    usage_metric: Option<Metric>,
    free_metric: Option<Metric>,
    total_metric: Option<Metric>,
) -> HardwareUsageInfo {
    let usage = if let Some(usage) = usage_metric { usage.value } else { 0.0 };
    let free = if let Some(free) = free_metric { free.value } else { 0.0 };
    let total = if let Some(total) = total_metric { total.value } else { 0.0 };
    HardwareUsageInfo {
        usage,
        free,
        status: if usage > ((usage + free) * 0.95) {
            HardwareInfoStatus::Critical
        } else if usage > ((usage + free) * 0.9) {
            HardwareInfoStatus::Warning
        } else {
            HardwareInfoStatus::Healthy
        },
    }
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

pub fn build_system_metrics(machine_metrics: &HashMap<String, Metric>) -> SystemMetrics {
    let system_metrics = SystemMetrics {
        cpu_cores: if let Some(cores) = machine_metrics.get(CORES_METRIC) {
            cores.value as u64
        } else {
            0
        },
        cpu_usage: if let Some(cpu) = machine_metrics.get(CPU_USAGE_METRIC) {
            cpu.value
        } else {
            0.0
        },
        memory_usage: if let Some(mem) = machine_metrics.get(MEMORY_USAGE_METRIC) {
            mem.value as u64
        } else {
            0
        },
        memory_free: if let Some(mem) = machine_metrics.get(MEMORY_FREE_METRIC) {
            mem.value as u64
        } else {
            0
        },
        memory_total: if let Some(mem) = machine_metrics.get(MEMORY_TOTAL_METRIC) {
            mem.value as u64
        } else {
            0
        },
        disk_free: if let Some(disk) = machine_metrics.get(DISK_TOTAL_METRIC) {
            let mut free = Vec::new();
            let mut i = 0;
            while let Some(value) = disk
                .attributes
                .as_ref()
                .and_then(|attrs| attrs.get(&format!("{}_{}", DISK_FREE_METRIC, i)))
                .and_then(|s| s.parse::<u64>().ok())
            {
                free.push(value);
                i += 1;
            }
            free
        } else {
            Vec::new()
        },
        disk_usage: if let Some(disk) = machine_metrics.get(DISK_TOTAL_METRIC) {
            let mut usage = Vec::new();
            let mut i = 0;
            while let Some(value) = disk
                .attributes
                .as_ref()
                .and_then(|attrs| attrs.get(&format!("{}_{}", DISK_USAGE_METRIC, i)))
                .and_then(|s| s.parse::<u64>().ok())
            {
                usage.push(value);
                i += 1;
            }
            usage
        } else {
            Vec::new()
        },
        disk_total: if let Some(disk) = machine_metrics.get(DISK_TOTAL_METRIC) {
            disk.value as u64
        } else {
            0
        },
        uptime: if let Some(uptime) = machine_metrics.get(UPTIME_METRIC) {
            uptime.value as u64
        } else {
            0
        },
    };
    system_metrics
}

pub fn convert_system_metrics(sys_info: &MachineData) -> Vec<Metrics> {
    let mut disk_attributes = vec![];
    for (i, disk) in sys_info.used_disk.iter().enumerate() {
        disk_attributes.push(MetricsAttribute {
            name: format!("{}_{}", DISK_USAGE_METRIC, i),
            value: disk.to_string(),
        });
    }
    for (i, disk) in sys_info.free_disk.iter().enumerate() {
        disk_attributes.push(MetricsAttribute {
            name: format!("{}_{}", DISK_FREE_METRIC, i),
            value: disk.to_string(),
        });
    }

    let sys_metrics = vec![
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
            attributes: disk_attributes,
        },
    ];
    sys_metrics
}
