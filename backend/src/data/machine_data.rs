use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    data::node_data::{build_avs_info, AvsInfo},
    db::{avs::Avs, machine::Machine, metric::Metric},
    error::BackendError,
};

const CORES_METRIC: &str = "cores";
const CPU_USAGE_METRIC: &str = "cpu_usage";
const MEMORY_USAGE_METRIC: &str = "ram_usage";
const MEMORY_FREE_METRIC: &str = "free_ram";
const DISK_USAGE_METRIC: &str = "disk_usage";
const DISK_FREE_METRIC: &str = "free_disk";

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
    pub cores: f64,
    pub cpu_usage: f64,
    pub memory_info: HardwareUsageInfo,
    pub disk_info: HardwareUsageInfo,
}

pub async fn build_machine_info(
    pool: &sqlx::PgPool,
    machine: &Machine,
    machine_metrics: HashMap<String, Metric>,
) -> Result<MachineInfoReport, BackendError> {
    let mut errors = vec![];

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

    let system_metrics = SystemMetrics {
        cores: if let Some(cores) = machine_metrics.get(CORES_METRIC) { cores.value } else { 0.0 },
        cpu_usage: if let Some(cpu) = machine_metrics.get(CPU_USAGE_METRIC) {
            cpu.value
        } else {
            0.0
        },
        memory_info,
        disk_info,
    };

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
) -> HardwareUsageInfo {
    let usage = if let Some(usage) = usage_metric { usage.value } else { 0.0 };
    let free = if let Some(free) = free_metric { free.value } else { 0.0 };
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
) -> Result<(Vec<Uuid>, Vec<Uuid>), BackendError> {
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
