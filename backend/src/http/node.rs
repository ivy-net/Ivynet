use std::collections::HashMap;

use ivynet_core::{ethers::types::Chain, node_type::NodeType};
use semver::Version;
use serde::Serialize;
use utoipa::ToSchema;

use crate::db::{avs_version::DbAvsVersionData, metric::Metric};

#[derive(Serialize, Debug, Clone)]
pub enum NodeError {
    NoOperatorId,
    ActiveSetNoDeployment,
    UnregisteredFromActiveSet,
}

#[derive(Serialize, Debug, Clone)]
pub struct NodeErrorInfo {
    name: NodeType,
    status: NodeError,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct AvsInfo {
    pub name: Option<String>,
    pub chain: Option<String>,
    pub version: Option<String>,
    pub active_set: Option<String>,
    pub operator_id: Option<String>,
    pub uptime: f64,
    pub performance_score: f64,
    pub updateable: Option<bool>,
}

const UPTIME_METRIC: &str = "uptime";
const RUNNING_METRIC: &str = "running";
const EIGEN_PERFORMANCE_METRIC: &str = "eigen_performance_score";

//TODO: THIS WILL PROBABLY CHANGE ONCE CLIENT IMPL IS DONE
pub async fn build_avs_info(pool: &sqlx::PgPool, metrics: HashMap<String, Metric>) -> AvsInfo {
    let running_metric = metrics.get(RUNNING_METRIC);
    let attrs = running_metric.and_then(|m| m.attributes.clone());
    let get_attr = |key| attrs.as_ref().and_then(|a| a.get(key).cloned());

    let name = get_attr("avs");
    let version = get_attr("version");
    let chain = get_attr("chain");

    //Like an onion
    let updateable = match (name.clone(), version.clone(), chain.clone()) {
        (Some(n), Some(v), Some(c)) => {
            let avs_name = NodeType::try_from(n.as_str()).ok();
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
        chain,
        active_set: get_attr("active_set"),
        operator_id: get_attr("operator_id"),
        uptime: metrics.get(UPTIME_METRIC).map_or(0.0, |m| m.value),
        performance_score: metrics.get(EIGEN_PERFORMANCE_METRIC).map_or(0.0, |m| m.value),
        updateable,
    }
}
