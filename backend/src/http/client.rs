use std::{collections::HashMap, str::FromStr};

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::NaiveDateTime;
use ivynet_core::{avs::names::AvsName, ethers::types::Chain};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    data::{self, NodeStatus},
    db::{
        avs::Avs,
        avs_version::DbAvsVersionData,
        log::{ContainerLog, LogLevel},
        machine::Machine,
        metric::Metric,
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
    pub avs_name: AvsName,
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
    pub updateable: Option<bool>,
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
    let machines = account.machines_and_avses(&state.pool).await?;

    let mut infos: Vec<Info> = vec![];

    for (machine, avses) in machines {
        for avs in avses {
            let metrics = Metric::get_organized_for_avs(
                &state.pool,
                avs.machine_id,
                &avs.avs_name.to_string(),
            )
            .await?;
            let info = build_node_info(&state.pool, &machine, &avs, metrics).await;
            infos.push(info);
        }
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

    let avses = account.avses(&state.pool).await?;

    let mut node_metrics_map = HashMap::new();

    for avs in &avses {
        node_metrics_map.insert(
            avs.machine_id,
            Metric::get_organized_for_avs(&state.pool, avs.machine_id, &avs.avs_name.to_string())
                .await?,
        );
    }

    let (running_nodes, idle_nodes) = data::categorize_running_nodes(node_metrics_map.clone());
    let (healthy_nodes, unhealthy_nodes) =
        data::categorize_node_health(running_nodes.clone(), node_metrics_map.clone());

    let avs_version_map = DbAvsVersionData::get_all_avs_version(&state.pool).await?;

    let (updateable_nodes, outdated_nodes) =
        data::categorize_updateable_nodes(running_nodes.clone(), node_metrics_map, avs_version_map);

    Ok(Status {
        error: Vec::new(),
        result: StatusReport {
            total_machines: avses.len(),
            healthy_machines: healthy_nodes.iter().map(|node| format!("{node:?}")).collect(),
            unhealthy_machines: unhealthy_nodes.iter().map(|node| format!("{node:?}")).collect(),
            idle_machines: idle_nodes.iter().map(|node| format!("{node:?}")).collect(),
            updateable_machines: updateable_nodes.iter().map(|node| format!("{node:?}")).collect(),
            erroring_machines: outdated_nodes.iter().map(|node| format!("{node:?}")).collect(),
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

    let avses = account.avses(&state.pool).await?;

    let mut node_metrics_map = HashMap::new();

    for avs in &avses {
        node_metrics_map.insert(
            avs.machine_id,
            Metric::get_organized_for_avs(&state.pool, avs.machine_id, &avs.avs_name.to_string())
                .await?,
        );
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

    let avses = account.avses(&state.pool).await?;

    let mut node_metrics_map = HashMap::new();

    for avs in &avses {
        node_metrics_map.insert(
            avs.machine_id,
            Metric::get_organized_for_avs(&state.pool, avs.machine_id, &avs.avs_name.to_string())
                .await?,
        );
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

    let avses = account.avses(&state.pool).await?;

    let mut node_metrics_map = HashMap::new();

    for avs in &avses {
        node_metrics_map.insert(
            avs.machine_id,
            Metric::get_organized_for_avs(&state.pool, avs.machine_id, &avs.avs_name.to_string())
                .await?,
        );
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
    authorize::verify_node_ownership(&account, State(state.clone()), id)
        .await?
        .set_name(&state.pool, &request.name)
        .await?;

    Ok(())
}

/// Delete a machine from the database
// TODO: We are already doing that. But there is too many things doing similar stuff
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
    authorize::verify_node_ownership(&account, State(state.clone()), id)
        .await?
        .delete(&state.pool)
        .await?;

    Ok(())
}

/// Get info on a specific node
#[utoipa::path(
    get,
    path = "/client/:machine_id/:avs_name",
    responses(
        (status = 200, body = Info),
        (status = 404)
    )
)]
pub async fn info(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path((machine_id, avs_name)): Path<(String, String)>,
) -> Result<Json<Info>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let machine =
        authorize::verify_node_ownership(&account, State(state.clone()), machine_id).await?;

    let metrics = Metric::get_organized_for_avs(&state.pool, machine.machine_id, &avs_name).await?;
    let avs = Avs::get_machines_avs(&state.pool, machine.machine_id, &avs_name)
        .await?
        .ok_or(BackendError::InvalidAvs)?;
    Ok(Json(build_node_info(&state.pool, &machine, &avs, metrics).await))
}

/// Get condensed metrics for a specific node
#[utoipa::path(
    get,
    path = "/client/:machine_id/:avs_name/metrics",
    responses(
        (status = 200, body = [Metric]),
        (status = 404)
    )
)]
pub async fn metrics_condensed(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path((machine_id, avs_name)): Path<(String, String)>,
    jar: CookieJar,
) -> Result<Json<Vec<Metric>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let machine =
        authorize::verify_node_ownership(&account, State(state.clone()), machine_id).await?;

    let metrics = Metric::get_all_for_avs(&state.pool, machine.machine_id, &avs_name).await?;

    let filtered_metrics = data::condense_metrics(&metrics)?;

    Ok(Json(filtered_metrics))
}

/// Get all metrics for a specific node
#[utoipa::path(
    get,
    path = "/client/:machine_id/:avs_name/metrics/all",
    responses(
        (status = 200, body = [Metric]),
        (status = 404)
    )
)]
pub async fn metrics_all(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path((machine_id, avs_name)): Path<(String, String)>,
    jar: CookieJar,
) -> Result<Json<Vec<Metric>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let machine =
        authorize::verify_node_ownership(&account, State(state.clone()), machine_id).await?;

    Ok(Metric::get_all_for_avs(&state.pool, machine.machine_id, &avs_name).await?.into())
}

#[utoipa::path(
    post,
    path = "/client/:machine_id/:avs_name/logs",
    responses(
        (status = 200, body = [ContainerLog]),
        (status = 404)
    ),
    params(
        ("log_level" = String, Query, description = "Optional log level filter. Valid values: debug, info, warning, error"),
        ("from" = String, Query, description = "Optional start timestamp"),
        ("to" = String, Query, description = "Optional end timestamp")
    )
)]
pub async fn logs(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path((machine_id, avs_name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ContainerLog>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let machine =
        authorize::verify_node_ownership(&account, State(state.clone()), machine_id).await?;

    let log_level = params
        .get("log_level")
        .map(|level| {
            LogLevel::from_str(level).map_err(|_| {
                BackendError::MalformedParameter("log_level".to_string(), level.clone())
            })
        })
        .transpose()?;

    let from = params.get("from").map(|s| s.parse::<i64>()).transpose().map_err(|_| {
        BackendError::MalformedParameter("from".to_string(), "Invalid timestamp".to_string())
    })?;
    let to = params.get("to").map(|s| s.parse::<i64>()).transpose().map_err(|_| {
        BackendError::MalformedParameter("to".to_string(), "Invalid timestamp".to_string())
    })?;

    if from.is_some() != to.is_some() {
        return Err(BackendError::MalformedParameter(
            "from/to".to_string(),
            "Both parameters must be present when querying by timestamp".to_string(),
        ));
    }

    let logs = ContainerLog::get_all_for_avs(
        &state.pool,
        machine.machine_id,
        &avs_name,
        from,
        to,
        log_level,
    )
    .await?;

    Ok(logs.into())
}

/// Get all data on every running avs for a specific node
#[utoipa::path(
    get,
    path = "/client/:machine_id/data/",
    responses(
        (status = 200, body = [Avs]),
        (status = 404)
    )
)]
pub async fn get_all_node_data(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path(machine_id): Path<String>,
    jar: CookieJar,
) -> Result<Json<Vec<Avs>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let machine =
        authorize::verify_node_ownership(&account, State(state.clone()), machine_id).await?;

    // Get all data for the node
    let nodes_data = Avs::get_machines_avs_list(&state.pool, machine.machine_id).await?;

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
pub async fn delete_machine_data(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    authorize::verify_node_ownership(&account, State(state.clone()), id)
        .await?
        .delete(&state.pool)
        .await?;

    Ok(())
}

// TODO: To be updated
/// Delete all data for a specific AVS running on a node
// #[utoipa::path(
//     delete,
//     path = "/client/:id/data/:avs/operator_id",
//     responses(
//         (status = 200),
//         (status = 404)
//     )
// )]
// pub async fn delete_avs_node_data(
//     headers: HeaderMap,
//     State(state): State<HttpState>,
//     Path((id, avs, operator_id)): Path<(String, String, String)>,
//     jar: CookieJar,
// ) -> Result<(), BackendError> {
//     let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
//     let machine = authorize::verify_node_ownership(&account, State(state.clone()), id).await?;
//     let avs_name = AvsName::try_from(&avs[..]).map_err(|_| BackendError::InvalidAvs)?;
//
//     let op_id: Address = operator_id.parse::<Address>().map_err(|_| BackendError::BadId)?;
//
//     avs.delete(&state.pool).await?;
//
//     Ok(())
// }

pub async fn build_node_info(
    pool: &sqlx::PgPool,
    machine: &Machine,
    avs: &Avs,
    node_metrics: HashMap<String, Metric>,
) -> Info {
    let last_checked = if let Some(running) = node_metrics.get(RUNNING_METRIC) {
        running.created_at
    } else {
        None
    };

    let avs_info = build_avs_info(
        pool,
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
            machine_id: format!("{:?}", machine.machine_id),
            name: machine.name.clone(),
            avs_name: avs.avs_name.clone(),
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
                deployed_avs: avs_info.await,
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

pub async fn build_avs_info(
    pool: &sqlx::PgPool,
    running_metric: Option<Metric>,
    performance_metric: Option<Metric>,
) -> AvsInfo {
    let attrs = running_metric.and_then(|m| m.attributes);
    let get_attr = |key| attrs.as_ref().and_then(|a| a.get(key).cloned());

    let name = get_attr("avs");
    let version = get_attr("version");
    let chain = get_attr("chain");

    //Like an onion
    let updateable = match (name.clone(), version.clone(), chain.clone()) {
        (Some(n), Some(v), Some(c)) => {
            let avs_name = AvsName::try_from(n.as_str()).ok();
            let avs_version = Version::parse(&v).ok();
            let avs_chain = c.parse::<Chain>().ok();

            match (avs_name, avs_version, avs_chain) {
                (Some(an), Some(av), Some(ac)) => {
                    let data = DbAvsVersionData::get_avs_version_with_chain(pool, &an, &ac)
                        .await
                        .unwrap_or(None);
                    match data {
                        Some(d) => Some(d.vd.latest_version > av),
                        None => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    };

    AvsInfo {
        name,
        version,
        active_set: get_attr("active_set"),
        operator_id: get_attr("operator_id"),
        chain,
        performance_score: performance_metric.map_or(0.0, |m| m.value),
        updateable,
    }
}
