use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use std::{collections::HashMap, str::FromStr};
use uuid::Uuid;

use crate::{
    data::{
        machine_data::{
            build_machine_info, get_machine_health, MachineInfoReport, MachineStatusReport,
        },
        node_data::{self, build_avs_info, AvsInfo},
    },
    db::{
        avs::Avs,
        log::{ContainerLog, LogLevel},
        metric::Metric,
    },
    error::BackendError,
};

use super::{authorize, HttpState};

/// Grab information for every node in the organization
#[utoipa::path(
    get,
    path = "/machines",
    responses(
        (status = 200, body = [Info]),
        (status = 404)
    )
)]
pub async fn machine(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<MachineInfoReport>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let machines = account.all_machines(&state.pool).await?;

    let mut info_reports: Vec<MachineInfoReport> = vec![];

    for machine in machines {
        let metrics = Metric::get_machine_metrics_only(&state.pool, machine.machine_id).await?;
        let info = build_machine_info(&state.pool, &machine, metrics).await?;
        info_reports.push(info);
    }

    Ok(Json(info_reports))
}

/// Get an overview of which nodes are healthy and unhealthy
#[utoipa::path(
    get,
    path = "/machines/status",
    responses(
        (status = 200, body = Status),
        (status = 404)
    )
)]
pub async fn status(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<MachineStatusReport>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let machines = account.all_machines(&state.pool).await?;
    let machine_ids = machines.iter().map(|m| m.machine_id).collect::<Vec<Uuid>>();

    let (healthy_list, unhealthy_list) = get_machine_health(&state.pool, machine_ids).await?;

    Ok(Json(MachineStatusReport {
        total_machines: machines.len(),
        healthy_machines: healthy_list.into_iter().map(|id| format!("{:?}", id)).collect(),
        unhealthy_machines: unhealthy_list.into_iter().map(|id| format!("{:?}", id)).collect(),
    }))
}

/// Get an overview of which machines are idle
#[utoipa::path(
    get,
    path = "/machine/idle",
    responses(
        (status = 200, body = Vec<String>),
        (status = 404)
    )
)]
pub async fn idle(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<String>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let machines = account.all_machines(&state.pool).await?;

    let mut idle: Vec<String> = vec![];
    for machine in &machines {
        let avses = Avs::get_machines_avs_list(&state.pool, machine.machine_id).await?;
        if avses.is_empty() {
            idle.push(format!("{:?}", machine.machine_id));
        }
    }

    Ok(Json(idle))
}

/// Get an overview of which nodes are unhealthy
#[utoipa::path(
    post,
    path = "/machine/unhealthy",
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
    let machines = account.all_machines(&state.pool).await?;
    let machine_ids = machines.iter().map(|m| m.machine_id).collect::<Vec<Uuid>>();

    let (_healthy_list, unhealthy_list) = get_machine_health(&state.pool, machine_ids).await?;
    Ok(Json(unhealthy_list.into_iter().map(|id| format!("{:?}", id)).collect()))
}

/// Get an overview of which nodes are healthy
#[utoipa::path(
    post,
    path = "/machine/healthy",
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
    let machines = account.all_machines(&state.pool).await?;
    let machine_ids = machines.iter().map(|m| m.machine_id).collect::<Vec<Uuid>>();

    let (healthy_list, _unhealthy_list) = get_machine_health(&state.pool, machine_ids).await?;
    Ok(Json(healthy_list.into_iter().map(|id| format!("{:?}", id)).collect()))
}

// #[derive(Deserialize, Debug, Clone, ToSchema)]
// pub struct NameChangeRequest {
//     pub name: String,
// }

/// --- Can't do this unless we can also update it in their config ---
// /// Set the name of a node
// #[utoipa::path(
//     post,
//     path = "/machine/:id/:name",
//     responses(
//         (status = 200),
//         (status = 404)
//     )
// )]
// pub async fn set_name(
//     headers: HeaderMap,
//     State(state): State<HttpState>,
//     jar: CookieJar,
//     Path(id): Path<String>,
//     Json(request): Json<NameChangeRequest>,
// ) -> Result<(), BackendError> {
//     let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
//     authorize::verify_node_ownership(&account, State(state.clone()), id)
//         .await?
//         .set_name(&state.pool, &request.name)
//         .await?;

//     Ok(())
// }

/// Delete a machine from the database
// TODO: We are already doing that. But there is too many things doing similar stuff
#[utoipa::path(
    delete,
    path = "/machine/:id",
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
    path = "/machine/:machine_id",
    responses(
        (status = 200, body = Info),
        (status = 404)
    )
)]
pub async fn info(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(machine_id): Path<String>,
) -> Result<Json<MachineInfoReport>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let machine =
        authorize::verify_node_ownership(&account, State(state.clone()), machine_id).await?;

    let metrics = Metric::get_machine_metrics_only(&state.pool, machine.machine_id).await?;
    Ok(Json(build_machine_info(&state.pool, &machine, metrics).await?))
}

/// Get condensed metrics for a specific node on a specific
#[utoipa::path(
    get,
    path = "/machine/:machine_id/:avs_name/metrics",
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

    let avses = Avs::get_machines_avs_list(&state.pool, machine.machine_id).await?;
    let avs = avses.iter().find(|avs| avs.avs_name == avs_name).ok_or(BackendError::InvalidAvs)?;
    let metrics = Metric::get_all_for_avs(&state.pool, machine.machine_id, &avs.avs_name).await?;

    let filtered_metrics = node_data::condense_metrics(avs.avs_type, &metrics)?;

    Ok(Json(filtered_metrics))
}

/// Get all metrics for a specific node
#[utoipa::path(
    get,
    path = "/machine/:machine_id/:avs_name/metrics/all",
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

    let avses = Avs::get_machines_avs_list(&state.pool, machine.machine_id).await?;
    let avs = avses.iter().find(|avs| avs.avs_name == avs_name).ok_or(BackendError::InvalidAvs)?;
    let metrics = Metric::get_all_for_avs(&state.pool, machine.machine_id, &avs.avs_name).await?;

    Ok(Json(metrics))
}

#[utoipa::path(
    post,
    path = "/machine/:machine_id/:avs_name/logs",
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
    path = "/machine/:machine_id/data/",
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
) -> Result<Json<Vec<AvsInfo>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let machine =
        authorize::verify_node_ownership(&account, State(state.clone()), machine_id).await?;

    // Get all data for the node
    let nodes = Avs::get_machines_avs_list(&state.pool, machine.machine_id).await?;

    let mut node_data = vec![];
    for node in nodes {
        let metrics =
            Metric::get_organized_for_avs(&state.pool, machine.machine_id, &node.avs_name).await?;
        node_data.push(build_avs_info(&state.pool, node.clone(), metrics).await);
    }

    Ok(Json(node_data))
}

/// Delete all data for a specific node
#[utoipa::path(
    delete,
    path = "/machine/:id",
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
