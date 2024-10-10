use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::NaiveDateTime;
use ivynet_core::{avs::names::AvsName, ethers::types::Address};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    data::{self, NodeStatus},
    db::{
        metric::Metric,
        node,
        node_data::{DbNodeData, NodeData},
    },
    error::BackendError,
};

use super::{authorize, HttpState};

const CPU_USAGE_METRIC: &str = "cpu_usage";
const MEMORY_USAGE_METRIC: &str = "ram_usage";
const DISK_USAGE_METRIC: &str = "disk_usage";
const UPTIME_METRIC: &str = "uptime";
const RUNNING_METRIC: &str = "running";

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

#[derive(Serialize, ToSchema, Clone, Debug, Default)]
pub struct Metrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub uptime: u64,
    pub deployed_avs: Option<String>,
    pub deployed_avs_chain: Option<String>,
    pub operators_pub_key: Option<String>,
    pub error: Vec<String>, // TODO: No idea what to do with it yet
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

    //TODO: Old (ie machines not pushing metrics) machines and updateable machines are not
    // implemented yet

    Ok(Status {
        error: Vec::new(),
        result: StatusReport {
            total_machines: machines.len(),
            healthy_machines: healthy_nodes.iter().map(|node| format!("{node:?}")).collect(),
            unhealthy_machines: unhealthy_nodes.iter().map(|node| format!("{node:?}")).collect(),
            idle_machines: idle_nodes.iter().map(|node| format!("{node:?}")).collect(),
            updateable_machines: Vec::new(), // TODO: How to get these?
            erroring_machines: Vec::new(),   // TODO: When we will solve error issues
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
    let (last_checked, deployed_avs, deployed_avs_chain, operators_pub_key) =
        if let Some(running) = metrics.get(RUNNING_METRIC) {
            if let Some(attributes) = &running.attributes {
                (
                    running.created_at,
                    attributes.get("avs").cloned(),
                    attributes.get("chain").cloned(),
                    attributes.get("operator_key").cloned(),
                )
            } else {
                (running.created_at, None, None, None)
            }
        } else {
            (None, None, None, None)
        };
    Ok(Info {
        error: Vec::new(),
        result: InfoReport {
            machine_id: id,
            name: machine.name,
            status: data::get_node_status(metrics.clone()), //TODO: This could still be improved
            metrics: Metrics {
                cpu_usage: if let Some(cpu) = metrics.get(CPU_USAGE_METRIC) {
                    cpu.value
                } else {
                    0.0
                },
                memory_usage: if let Some(ram) = metrics.get(MEMORY_USAGE_METRIC) {
                    ram.value
                } else {
                    0.0
                },
                disk_usage: if let Some(disk) = metrics.get(DISK_USAGE_METRIC) {
                    disk.value
                } else {
                    0.0
                },
                uptime: if let Some(uptime) = metrics.get(UPTIME_METRIC) {
                    uptime.value as u64
                } else {
                    0
                },
                deployed_avs,
                deployed_avs_chain,
                operators_pub_key,
                error: Vec::new(),
            },

            last_checked,
        },
    }
    .into())
}

/// Get all data on every running avs for a specific node
#[utoipa::path(
    get,
    path = "/client/:id/data/",
    responses(
        (status = 200, body = Metric),
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
        (status = 200, body = Metric),
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
    let nodes_data = DbNodeData::get_node_data(&state.pool, &node_id, &avs_name).await?;

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

    DbNodeData::delete_all(&state.pool, &node_id).await?;

    Ok(())
}

/// Delete all data for a specific AVS running on a node
#[utoipa::path(
    delete,
    path = "/client/:id/data/:avs",
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn delete_avs_node_data(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path((id, avs)): Path<(String, String)>,
    jar: CookieJar,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let node_id =
        authorize::verify_node_ownership(&account, State(state.clone()), Path(id)).await?;
    let avs_name = AvsName::from(&avs);

    DbNodeData::delete_avs(&state.pool, &node_id, &avs_name).await?;

    Ok(())
}
