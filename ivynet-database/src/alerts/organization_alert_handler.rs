use async_trait::async_trait;
use ethers::types::Address;
use ivynet_alerts::{Alert, Channel, SendState};
use ivynet_notifications::{NotificationDispatcher, NotificationDispatcherError};
use sqlx::{types::Uuid, PgPool};
use std::sync::Arc;

use crate::{
    alerts::organization_alerts_active::{NewOrganizationAlert, OrganizationActiveAlert},
    eigen_avs_metadata::{EigenAvsMetadata, MetadataContent},
    error::DatabaseError,
    Organization,
};

use super::{
    alert_db::AlertDb,
    alert_handler::{ActiveAlert, AlertHandler, NewAlert},
};

#[derive(Debug, thiserror::Error)]
pub enum OrganizationAlertError {
    #[error(transparent)]
    DbError(#[from] DatabaseError),
    #[error(transparent)]
    NotificationError(#[from] NotificationDispatcherError),
    #[error(transparent)]
    SqxlError(#[from] sqlx::Error),
}

impl NewAlert for NewOrganizationAlert {
    fn get_id(&self) -> Uuid {
        self.id
    }

    fn get_alert_type(&self) -> Alert {
        self.alert_type.clone()
    }

    fn set_send_state(&mut self, channel: Channel, state: SendState) {
        match channel {
            Channel::Telegram => self.telegram_send = state,
            Channel::Email => self.sendgrid_send = state,
            Channel::PagerDuty => self.pagerduty_send = state,
        }
    }
}

impl ActiveAlert for OrganizationActiveAlert {
    fn get_id(&self) -> Uuid {
        self.alert_id
    }

    fn get_alert_type(&self) -> Alert {
        self.alert_type.clone()
    }
}

#[derive(Clone)]
pub struct OrganizationAlertHandler {
    pub dispatcher: Arc<NotificationDispatcher<AlertDb>>,
    db_executor: PgPool,
}

impl OrganizationAlertHandler {
    pub fn new(dispatcher: Arc<NotificationDispatcher<AlertDb>>, db_executor: PgPool) -> Self {
        Self { dispatcher, db_executor }
    }

    pub async fn handle_new_eigen_avs_alerts(
        &self,
        pool: &PgPool,
        avs_address: &Address,
        block_number: u64,
        log_index: u64,
        metadata_uri: &str,
        metadata_content: &MetadataContent,
    ) -> Result<(), OrganizationAlertError> {
        let count = EigenAvsMetadata::search_for_avs(
            pool,
            *avs_address,
            metadata_uri.to_owned(),
            metadata_content.name.clone().unwrap_or_default(),
            metadata_content.website.clone().unwrap_or_default(),
            metadata_content.twitter.clone().unwrap_or_default(),
        )
        .await
        .map_err(|e| {
            OrganizationAlertError::DbError(DatabaseError::FailedMetadata(format!(
                "Failed to get count of metadata: {}",
                e
            )))
        })?;

        println!("count: {:?}", count);

        let organization_ids = Organization::get_all_ids(pool).await?;
        let is_update = count > 0;

        tracing::debug!(
            "AVS {} - sending {} alert",
            if is_update { "already registered" } else { "not registered" },
            if is_update { "update" } else { "new" }
        );

        let mut new_alerts = Vec::new();
        let alert_type = if is_update {
            Alert::UpdatedEigenAvs {
                address: *avs_address,
                block_number,
                log_index,
                name: metadata_content.name.clone().unwrap_or_default(),
                metadata_uri: metadata_uri.to_string(),
                description: metadata_content.description.clone().unwrap_or_default(),
                website: metadata_content.website.clone().unwrap_or_default(),
                logo: metadata_content.logo.clone().unwrap_or_default(),
                twitter: metadata_content.twitter.clone().unwrap_or_default(),
            }
        } else {
            Alert::NewEigenAvs {
                address: *avs_address,
                block_number,
                log_index,
                name: metadata_content.name.clone().unwrap_or_default(),
                metadata_uri: metadata_uri.to_string(),
                description: metadata_content.description.clone().unwrap_or_default(),
                website: metadata_content.website.clone().unwrap_or_default(),
                logo: metadata_content.logo.clone().unwrap_or_default(),
                twitter: metadata_content.twitter.clone().unwrap_or_default(),
            }
        };

        println!("alert_type: {:?}", alert_type);

        for organization_id in organization_ids {
            let alert = NewOrganizationAlert::new(organization_id, alert_type.clone());
            new_alerts.push(alert);
        }

        let existing_alerts = OrganizationActiveAlert::all_alerts_by_org(pool, 1).await?;
        let new_alerts = self.filter_duplicate_alerts(new_alerts, existing_alerts).await?;
        OrganizationActiveAlert::insert_many(pool, &new_alerts).await?;

        for alert in new_alerts {
            println!("alert: {:#?}", alert);
            self.send_notifications(&mut vec![alert.clone()], alert.organization_id as u64, None)
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl AlertHandler for OrganizationAlertHandler {
    type Error = OrganizationAlertError;
    type NewAlertType = NewOrganizationAlert;
    type ActiveAlertType = OrganizationActiveAlert;

    fn get_dispatcher(&self) -> &Arc<NotificationDispatcher<AlertDb>> {
        &self.dispatcher
    }

    fn get_db_pool(&self) -> &PgPool {
        &self.db_executor
    }

    async fn filter_duplicate_alerts(
        &self,
        incoming_alerts: Vec<Self::NewAlertType>,
        existing_alerts: Vec<Self::ActiveAlertType>,
    ) -> Result<Vec<Self::NewAlertType>, Self::Error> {
        let existing_ids = existing_alerts.iter().map(|alert| alert.alert_id).collect::<Vec<_>>();

        println!("existing_ids: {:#?}", existing_ids);
        println!("incoming_alerts: {:#?}", incoming_alerts);

        let filtered = incoming_alerts
            .into_iter()
            .filter(|alert| !existing_ids.contains(&alert.id))
            .collect::<Vec<_>>();

        println!("filtered: {:#?}", filtered);

        println!("--------------    ");

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    // use ivynet_notifications::{NotificationConfig, SendgridSpecificTemplates, SendgridTemplates};

    // use super::*;

    // fn dummy_config_fixture() -> NotificationConfig {
    //     let specific_templates = SendgridSpecificTemplates {
    //         custom: "test".to_string(),
    //         unreg_active_set: "test".to_string(),
    //         machine_not_responding: "test".to_string(),
    //         node_not_running: "test".to_string(),
    //         no_chain_info: "test".to_string(),
    //         no_metrics: "test".to_string(),
    //         no_operator: "test".to_string(),
    //         hw_res_usage: "test".to_string(),
    //         low_perf: "test".to_string(),
    //         needs_update: "test".to_string(),
    //         new_eigen_avs: "test".to_string(),
    //         updated_eigen_avs: "test".to_string(),
    //     };

    //     NotificationConfig {
    //         telegram_token: "test".to_string(),
    //         sendgrid_key: "test".to_string(),
    //         sendgrid_from: "test".to_string(),
    //         sendgrid_templates: SendgridTemplates::Specific(Box::new(specific_templates)),
    //     }
    // }

    // fn handler_fixture(pool: &PgPool) -> OrganizationAlertHandler {
    //     OrganizationAlertHandler::new(
    //         Arc::new(NotificationDispatcher::new(
    //             dummy_config_fixture(),
    //             AlertDb::new(pool.clone()),
    //         )),
    //         pool.clone(),
    //     )
    // }

    // #[sqlx::test(
    //     migrations = "../migrations",
    //     fixtures(
    //         "../../fixtures/new_user_registration.sql",
    //         "../../fixtures/node_alerts_active.sql",
    //     )
    // )]
    // #[ignore]
    // async fn test_filter_duplicate_alerts(pool: PgPool) {
    //     let handler = handler_fixture(&pool);
    //     let machine_id = Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();
    //     let node_name = "test_node_123123".to_string();
    //     let alert_type_1 = Alert::Custom {
    //         node_name: node_name.clone(),
    //         node_type: "test_type".to_string(),
    //         extra_data: serde_json::Value::String("runtime_alert_fixture_1".to_string()),
    //     };

    //     todo!()
    // }
}
