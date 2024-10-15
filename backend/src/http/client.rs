use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::NaiveDateTime;
use ivynet_core::{avs::names::AvsName, ethers::types::Address};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    data::{self, NodeStatus},
    db::{
        avs_data::DbAvsData,
        log::{ContainerLog, LogLevel},
        metric::Metric,
        node,
        node_data::{DbNodeData, NodeData},
    },
    error::BackendError,
};

use super::{authorize, HttpState};

const CPU_USAGE_METRIC: &str = "cpu_usage";
const MEMORY_USAGE_METRIC: &str = "ram_usage";
const MEMORY_FREE_METRIC: &str = "free_ram";
const DISK_USAGE_METRIC: &str = "disk_usage";
const DISK_FREE_METRIC: &str = "free_disk";
const UPTIME_METRIC: &str = "uptime";
const RUNNING_METRIC: &str = "running";
const EIGEN_PERFORMANCE_METRIC: &str = "eigen_performance_score";

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub struct NameChangeRequest {
    pub name: String,
}

// TODO: We still need to define how we handle errors in avs
#[derive(Serialize, Debug, Clone)]
pub enum StatusError {}

#[derive(Serialize, ToSchema, Clone, Debug, Default)]
pub struct StatusReport {
    pub total_machines: usize,
    pub healthy_machines: Vec<String>,
    pub unhealthy_machines: Vec<String>,
    pub idle_machines: Vec<String>,
    pub updateable_machines: Vec<String>,
    pub erroring_machines: Vec<String>,
}

#[derive(Serialize, ToSchema, Clone, Debug, Default)]
pub struct Status {
    pub error: Vec<StatusError>,
    pub result: StatusReport,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct Info {
    pub error: Vec<StatusError>,
    pub result: InfoReport,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct InfoReport {
    pub machine_id: String,
    pub name: String,
    pub status: NodeStatus,
    pub metrics: Metrics,
    pub last_checked: Option<NaiveDateTime>,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct Metrics {
    pub cpu_usage: f64,
    pub memory_info: HardwareUsageInfo,
    pub disk_info: HardwareUsageInfo,
    pub uptime: u64,
    pub deployed_avs: AvsInfo,
    pub error: Vec<String>, // TODO: No idea what to do with it yet
}

#[derive(Debug, Deserialize, ToSchema, Clone)]
pub struct LogFilter {
    log_level: Option<LogLevel>,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct HardwareUsageInfo {
    pub usage: f64,
    pub free: f64,
    pub status: HardwareInfoStatus,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub enum HardwareInfoStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct AvsInfo {
    pub name: Option<String>,
    pub chain: Option<String>,
    pub version: Option<String>,
    pub active_set: Option<String>,
    pub operator_id: Option<String>,
    pub performance_score: f64,
}

/// Grab information for every node in the organization
#[utoipa::path(
    get,
    path = "/client",
    responses(
        (status = 200, body = [Info]),
        (status = 404)
    )
)]
pub async fn client(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<Info>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let machines: Vec<node::Node> = account.nodes(&state.pool).await?;

    let mut infos: Vec<Info> = vec![];

    for machine in machines {
        let metrics = Metric::get_organized_for_node(&state.pool, &machine).await?;
        let info = build_node_info(machine, metrics);
        infos.push(info);
    }

    Ok(Json(infos))
}

/// Get an overview of which nodes are healthy, unhealthy, idle, and erroring
#[utoipa::path(
    get,
    path = "/client/status",
    responses(
        (status = 200, body = Status),
        (status = 404)
    )
)]
pub async fn status(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Status>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let machines = account.nodes(&state.pool).await?;

    let mut node_metrics_map = HashMap::new();

    for machine in &machines {
        node_metrics_map
            .insert(machine.node_id, Metric::get_organized_for_node(&state.pool, machine).await?);
    }

    let (running_nodes, idle_nodes) = data::categorize_running_nodes(node_metrics_map.clone());
    let (healthy_nodes, unhealthy_nodes) =
        data::categorize_node_health(running_nodes.clone(), node_metrics_map.clone());

    let avs_versions = DbAvsData::get_all_avs_data(&state.pool).await?;
    let avs_version_map: HashMap<AvsName, Version> =
        avs_versions.iter().map(|avs| (avs.avs_name.clone(), avs.avs_version.clone())).collect();

    let updateable_nodes = data::catgegorize_updateable_nodes(
        running_nodes.clone(),
        node_metrics_map,
        avs_version_map,
    );

    Ok(Status {
        error: Vec::new(),
        result: StatusReport {
            total_machines: machines.len(),
            healthy_machines: healthy_nodes.iter().map(|node| format!("{node:?}")).collect(),
            unhealthy_machines: unhealthy_nodes.iter().map(|node| format!("{node:?}")).collect(),
            idle_machines: idle_nodes.iter().map(|node| format!("{node:?}")).collect(),
            updateable_machines: updateable_nodes.iter().map(|node| format!("{node:?}")).collect(),
            erroring_machines: Vec::new(), // TODO: When we will solve error issues
        },
    }
    .into())
}

/// Get an overview of which nodes are idle
#[utoipa::path(
    get,
    path = "/client/idle",
    responses(
        (status = 200, body = Vec<String>),
        (status = 404)
    )
)]
pub async fn idling(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<String>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let machines = account.nodes(&state.pool).await?;

    let mut node_metrics_map = HashMap::new();

    for machine in &machines {
        node_metrics_map
            .insert(machine.node_id, Metric::get_organized_for_node(&state.pool, machine).await?);
    }

    let (_, idle_nodes) = data::categorize_running_nodes(node_metrics_map.clone());

    Ok(Json(idle_nodes.iter().map(|node| format!("{node:?}")).collect()))
}

/// Get an overview of which nodes are unhealthy
#[utoipa::path(
    post,
    path = "/client/unhealthy",
    responses(
        (status = 200, body = Vec<String>),
        (status = 404)
    )
)]
pub async fn unhealthy(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<String>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let machines = account.nodes(&state.pool).await?;

    let mut node_metrics_map = HashMap::new();

    for machine in &machines {
        node_metrics_map
            .insert(machine.node_id, Metric::get_organized_for_node(&state.pool, machine).await?);
    }

    let (running_nodes, _) = data::categorize_running_nodes(node_metrics_map.clone());
    let (_, unhealthy_nodes) =
        data::categorize_node_health(running_nodes.clone(), node_metrics_map.clone());

    Ok(Json(unhealthy_nodes.iter().map(|node| format!("{node:?}")).collect()))
}

/// Get an overview of which nodes are healthy
#[utoipa::path(
    post,
    path = "/client/healthy",
    responses(
        (status = 200, body = Vec<String>),
        (status = 404)
    )
)]
pub async fn healthy(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<String>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let machines = account.nodes(&state.pool).await?;

    let mut node_metrics_map = HashMap::new();

    for machine in &machines {
        node_metrics_map
            .insert(machine.node_id, Metric::get_organized_for_node(&state.pool, machine).await?);
    }

    let (running_nodes, _) = data::categorize_running_nodes(node_metrics_map.clone());
    let (healthy_nodes, _) =
        data::categorize_node_health(running_nodes.clone(), node_metrics_map.clone());

    Ok(Json(healthy_nodes.iter().map(|node| format!("{node:?}")).collect()))
}

/// Set the name of a node
#[utoipa::path(
    post,
    path = "/client/:id/:name",
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn set_name(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<String>,
    Json(request): Json<NameChangeRequest>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let address = id.parse::<Address>().map_err(|_| BackendError::BadId)?;
    let machine = node::DbNode::get(&state.pool, &address).await?;
    if machine.organization_id != account.organization_id || !account.role.can_write() {
        return Err(BackendError::Unauthorized);
    }

    node::DbNode::set_name(&state.pool, &address, &request.name).await?;
    node::DbNode::delete(&state.pool, &address).await?;

    Ok(())
}

/// Delete a node from the database
#[utoipa::path(
    delete,
    path = "/client/:id",
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn delete(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let address = id.parse::<Address>().map_err(|_| BackendError::BadId)?;
    let machine = node::DbNode::get(&state.pool, &address).await?;
    if machine.organization_id != account.organization_id || !account.role.can_write() {
        return Err(BackendError::Unauthorized);
    }

    node::DbNode::delete(&state.pool, &address).await?;

    Ok(())
}

/// Get info on a specific node
#[utoipa::path(
    get,
    path = "/client/:id",
    responses(
        (status = 200, body = Info),
        (status = 404)
    )
)]
pub async fn info(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<Json<Info>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let address = id.parse::<Address>().map_err(|_| BackendError::BadId)?;
    let machine = node::DbNode::get(&state.pool, &address).await?;
    if machine.organization_id != account.organization_id {
        return Err(BackendError::Unauthorized);
    }

    let metrics = Metric::get_organized_for_node(&state.pool, &machine).await?;
    Ok(Json(build_node_info(machine, metrics)))
}

/// Get condensed metrics for a specific node
#[utoipa::path(
    get,
    path = "/client/:id/metrics",
    responses(
        (status = 200, body = [Metric]),
        (status = 404)
    )
)]
pub async fn metrics_condensed(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path(id): Path<String>,
    jar: CookieJar,
) -> Result<Json<Vec<Metric>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let node_id =
        authorize::verify_node_ownership(&account, State(state.clone()), Path(id)).await?;

    let metrics = Metric::get_all_for_node(&state.pool, node_id).await?;

    let filtered_metrics = data::condense_metrics(&metrics)?;

    Ok(Json(filtered_metrics))
}

/// Get all metrics for a specific node
#[utoipa::path(
    get,
    path = "/client/:id/metrics/all",
    responses(
        (status = 200, body = [Metric]),
        (status = 404)
    )
)]
pub async fn metrics_all(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path(id): Path<String>,
    jar: CookieJar,
) -> Result<Json<Vec<Metric>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let node_id =
        authorize::verify_node_ownership(&account, State(state.clone()), Path(id)).await?;

    Ok(Metric::get_all_for_node(&state.pool, node_id).await?.into())
}

#[utoipa::path(
    post,
    path = "/client/:id/logs",
    responses(
        (status = 200, body = [ContainerLog]),
        (status = 404)
    ),
    params(
        ("log_level" = Option<LogLevel>, Query, description = "Optional log level filter")
    )
)]
pub async fn logs(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<String>,
    Query(log_filter): Query<LogFilter>,
) -> Result<Json<Vec<ContainerLog>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let node_id =
        authorize::verify_node_ownership(&account, State(state.clone()), Path(id)).await?;

    // Fetch logs, optionally filtering by log level
    let logs = if let Some(log_level) = log_filter.log_level {
        ContainerLog::get_all_for_node_with_log_level(&state.pool, node_id, log_level).await?
    } else {
        ContainerLog::get_all_for_node(&state.pool, node_id).await?
    };

    Ok(logs.into())
}

#[utoipa::path(
    post,
    path = "/client/:id/logs/:from/:to",
    responses(
        (status = 200, body = [ContainerLog]),
        (status = 404)
    ),
    params(
        ("log_level" = Option<LogLevel>, Query, description = "Optional log level filter")
    )
)]
pub async fn logs_between(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path((id, from, to)): Path<(String, i64, i64)>,
    Query(log_filter): Query<LogFilter>,
) -> Result<Json<Vec<ContainerLog>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let node_id =
        authorize::verify_node_ownership(&account, State(state.clone()), Path(id)).await?;

    // Fetch logs between timestamps, optionally filtering by log level
    let logs = if let Some(log_level) = log_filter.log_level {
        ContainerLog::get_all_for_node_between_timestamps_with_log_level(
            &state.pool,
            node_id,
            from,
            to,
            log_level,
        )
        .await?
    } else {
        ContainerLog::get_all_for_node_between_timestamps(&state.pool, node_id, from, to).await?
    };

    Ok(logs.into())
}

/// Get all data on every running avs for a specific node
#[utoipa::path(
    get,
    path = "/client/:id/data/",
    responses(
        (status = 200, body = [NodeData]),
        (status = 404)
    )
)]
pub async fn get_all_node_data(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path(id): Path<String>,
    jar: CookieJar,
) -> Result<Json<Vec<NodeData>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let node_id =
        authorize::verify_node_ownership(&account, State(state.clone()), Path(id)).await?;

    // Get all data for the node
    let nodes_data = DbNodeData::get_all_node_data(&state.pool, &node_id).await?;

    Ok(Json(nodes_data))
}

/// Get all data on a specific AVS running on a node
/// Keep in mind, a node could run the same avs multiple times
/// assuming they are using different operator keys
#[utoipa::path(
    get,
    path = "/client/:id/data/:avs",
    responses(
        (status = 200, body = [NodeData]),
        (status = 404)
    )
)]
pub async fn get_node_data_for_avs(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path((id, avs)): Path<(String, String)>,
    jar: CookieJar,
) -> Result<Json<Vec<NodeData>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let node_id =
        authorize::verify_node_ownership(&account, State(state.clone()), Path(id)).await?;
    let avs_name = AvsName::from(&avs);

    // Get all data for the node
    let nodes_data = DbNodeData::get_avs_node_data(&state.pool, &node_id, &avs_name).await?;

    Ok(Json(nodes_data))
}

/// Delete all data for a specific node
#[utoipa::path(
    delete,
    path = "/client/:id/data",
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn delete_node_data(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let node_id =
        authorize::verify_node_ownership(&account, State(state.clone()), Path(id)).await?;

    DbNodeData::delete_all_node_data(&state.pool, &node_id).await?;

    Ok(())
}

/// Delete all data for a specific AVS running on a node
#[utoipa::path(
    delete,
    path = "/client/:id/data/:avs/:operator_id",
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn delete_avs_node_data(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path((id, avs, operator_id)): Path<(String, String, String)>,
    jar: CookieJar,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let _node_id =
        authorize::verify_node_ownership(&account, State(state.clone()), Path(id)).await?;
    let avs_name = AvsName::from(&avs);

    let op_id: Address = operator_id.parse::<Address>().map_err(|_| BackendError::BadId)?;

    DbNodeData::delete_avs_operator_data(&state.pool, &op_id, &avs_name).await?;

    Ok(())
}

pub fn build_node_info(node: node::Node, node_metrics: HashMap<String, Metric>) -> Info {
    let last_checked = if let Some(running) = node_metrics.get(RUNNING_METRIC) {
        running.created_at
    } else {
        None
    };

    let avs_info = build_avs_info(
        node_metrics.get(RUNNING_METRIC).cloned(),
        node_metrics.get(EIGEN_PERFORMANCE_METRIC).cloned(),
    );

    let memory_info = build_hardware_info(
        node_metrics.get(MEMORY_USAGE_METRIC).cloned(),
        node_metrics.get(MEMORY_FREE_METRIC).cloned(),
    );

    let disk_info = build_hardware_info(
        node_metrics.get(DISK_USAGE_METRIC).cloned(),
        node_metrics.get(DISK_FREE_METRIC).cloned(),
    );

    Info {
        error: Vec::new(),
        result: InfoReport {
            machine_id: format!("{:?}", node.node_id),
            name: node.name,
            status: data::get_node_status(node_metrics.clone()),
            metrics: Metrics {
                cpu_usage: if let Some(cpu) = node_metrics.get(CPU_USAGE_METRIC) {
                    cpu.value
                } else {
                    0.0
                },
                memory_info,
                disk_info,
                uptime: if let Some(uptime) = node_metrics.get(UPTIME_METRIC) {
                    uptime.value as u64
                } else {
                    0
                },
                deployed_avs: avs_info,
                error: Vec::new(),
            },

            last_checked,
        },
    }
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

pub fn build_avs_info(
    running_metric: Option<Metric>,
    performance_metric: Option<Metric>,
) -> AvsInfo {
    let mut name = None;
    let mut version = None;
    let mut active_set = None;
    let mut operator_id = None;
    let mut chain = None;

    if let Some(running) = running_metric {
        if let Some(attributes) = &running.attributes {
            name = attributes.get("avs").cloned();
            version = attributes.get("version").cloned();
            chain = attributes.get("chain").cloned();
            operator_id = attributes.get("operator_id").cloned();
            active_set = attributes.get("active_set").cloned();
        }
    }

    AvsInfo {
        name,
        version,
        active_set,
        operator_id,
        chain,
        performance_score: if let Some(performance) = performance_metric {
            performance.value
        } else {
            0.0
        },
    }
}
