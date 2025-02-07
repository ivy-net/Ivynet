use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use db::alerts::alerts_active::ActiveAlert;
use sqlx::Pool;

use crate::error::BackendError;

use super::HttpState;

pub async fn active_alerts(
    header: HeaderMap,
    State(state): State<Arc<HttpState>>,
    jar: CookieJar,
) -> Result<Json<Vec<ActiveAlert>>, BackendError> {
    todo!()
}

pub async fn acknowledge_alert(
    header: HeaderMap,
    State(state): State<Arc<HttpState>>,
    jar: CookieJar,
    Query(params): Query<AcknowledgeAlertParams>,
) -> Result<Json<ActiveAlert>, BackendError> {
    todo!()
}

pub struct AcknowledgeAlertParams {
    pub alert_id: u64,
}

pub async fn historical_alerts(
    header: HeaderMap,
    State(state): State<Arc<HttpState>>,
    jar: CookieJar,
    Query(params): Query<HistoricalAlertParams>,
) -> Result<Json<Vec<ActiveAlert>>, BackendError> {
    todo!()
}

#[derive(Debug, Clone)]
pub struct HistoricalAlertParams {
    pub from: u64,
    pub to: u64,
}

impl Default for HistoricalAlertParams {
    fn default() -> Self {
        Self { from: 0, to: u64::MAX }
    }
}
