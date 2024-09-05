use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::NaiveDateTime;
use ivynet_core::ethers::types::Address;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    db::{metric, node},
    error::BackendError,
};

use super::{authorize, HttpState};

const EIGEN_PERFORMANCE_METRIC: &str = "eigen_performance_score";
const CPU_USAGE_METRIC: &str = "cpu_usage";
const MEMORY_USAGE_METRIC: &str = "ram_usage";
const DISK_USAGE_METRIC: &str = "disk_usage";
const UPTIME_METRIC: &str = "uptime";
const RUNNING_METRIC: &str = "running";

const EIGEN_PERFORMANCE_HEALTHY_THRESHOLD: f64 = 80.0;

// TODO: We still need to define how we handle errors in avs
#[derive(Serialize, Debug, Clone)]
pub enum StatusError {}

#[derive(Serialize, ToSchema, Clone, Debug, Default)]
pub struct StatusReport {
    pub total_machines: usize,
    pub healthy_machines: usize,
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

#[derive(Serialize, ToSchema, Clone, Debug, Default)]
pub struct Info {
    pub error: Vec<StatusError>,
    pub result: InfoReport,
}

#[derive(Serialize, ToSchema, Clone, Debug, Default)]
pub struct InfoReport {
    pub machine_id: String,
    pub status: String,
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

#[utoipa::path(
    post,
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

    let mut metrics = HashMap::new();

    for machine in &machines {
        metrics.insert(
            machine.node_id,
            metric::Metric::get_organized_for_node(&state.pool, machine).await?,
        );
    }

    Ok(Status {
        error: Vec::new(),
        result: StatusReport {
            total_machines: machines.len(),
            healthy_machines: metrics
                .iter()
                .filter_map(|(machine, metrics)| {
                    if let Some(performace) = metrics.get(EIGEN_PERFORMANCE_METRIC) {
                        if performace.value >= EIGEN_PERFORMANCE_HEALTHY_THRESHOLD {
                            return Some(machine);
                        }
                    }
                    None
                })
                .collect::<Vec<_>>()
                .len(),
            unhealthy_machines: metrics
                .iter()
                .filter_map(|(machine, metrics)| {
                    if let Some(performace) = metrics.get(EIGEN_PERFORMANCE_METRIC) {
                        if performace.value >= EIGEN_PERFORMANCE_HEALTHY_THRESHOLD {
                            return None;
                        }
                    }
                    Some(format!("{machine:?}"))
                })
                .collect::<Vec<_>>(),
            idle_machines: metrics
                .iter()
                .filter_map(|(machine, metrics)| {
                    if let Some(performace) = metrics.get(RUNNING_METRIC) {
                        if performace.value == 1.0 {
                            return None;
                        }
                    }
                    Some(format!("{machine:?}"))
                })
                .collect::<Vec<_>>(),
            updateable_machines: Vec::new(), // TODO: How to get these?
            erroring_machines: Vec::new(),   // TODO: When we will solve error issues
        },
    }
    .into())
}

#[utoipa::path(
    post,
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

    let mut metrics = HashMap::new();

    for machine in &machines {
        metrics.insert(
            machine.node_id,
            metric::Metric::get_organized_for_node(&state.pool, machine).await?,
        );
    }

    Ok(metrics
        .iter()
        .filter_map(|(machine, metrics)| {
            if let Some(performace) = metrics.get(RUNNING_METRIC) {
                if performace.value == 1.0 {
                    return None;
                }
            }
            Some(format!("{machine:?}"))
        })
        .collect::<Vec<_>>()
        .into())
}

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

    let mut metrics = HashMap::new();

    for machine in &machines {
        metrics.insert(
            machine.node_id,
            metric::Metric::get_organized_for_node(&state.pool, machine).await?,
        );
    }

    Ok(metrics
        .iter()
        .filter_map(|(machine, metrics)| {
            if let Some(performace) = metrics.get(EIGEN_PERFORMANCE_METRIC) {
                if performace.value >= EIGEN_PERFORMANCE_HEALTHY_THRESHOLD {
                    return None;
                }
            }
            Some(format!("{machine:?}"))
        })
        .collect::<Vec<_>>()
        .into())
}

#[utoipa::path(
    post,
    path = "/client/info/{id}",
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

    let metrics = metric::Metric::get_organized_for_node(&state.pool, &machine).await?;
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
            status: "Healthy".to_owned(), // TODO: This is wrong. We don't know what potential
            // statuses are
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
