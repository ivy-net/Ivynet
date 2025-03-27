use std::sync::Arc;

use ivynet_alerts::{Alert, SendState};
use ivynet_grpc::messages::MachineData;
use ivynet_notifications::{Channel, NotificationDispatcher, NotificationDispatcherError};
use sqlx::{types::Uuid, PgPool};

use async_trait::async_trait;

use crate::{
    alerts::{
        alert_db::AlertDb,
        alert_handler::{ActiveAlert, AlertHandler, NewAlert},
    },
    data::machine_data::build_system_metrics_from_machine_data,
    error::DatabaseError,
    Avs, Machine,
};

use super::alerts_active::{MachineActiveAlert, NewMachineAlert};

#[derive(Debug, thiserror::Error)]
pub enum MachineAlertError {
    #[error(transparent)]
    DbError(#[from] DatabaseError),
    #[error(transparent)]
    NotificationError(#[from] NotificationDispatcherError),
    #[error(transparent)]
    SqxlError(#[from] sqlx::Error),
}

impl NewAlert for NewMachineAlert {
    fn get_id(&self) -> Uuid {
        self.id
    }

    fn get_alert_type(&self) -> Alert {
        self.alert_type.clone()
    }

    fn set_send_state(&mut self, channel: &Channel, state: SendState) {
        match channel {
            Channel::Telegram(_) => self.telegram_send = state,
            Channel::Email(_) => self.sendgrid_send = state,
            Channel::PagerDuty(_) => self.pagerduty_send = state,
        }
    }
}

impl ActiveAlert for MachineActiveAlert {
    fn get_id(&self) -> Uuid {
        self.alert_id
    }

    fn get_alert_type(&self) -> Alert {
        self.alert_type.clone()
    }
}

#[derive(Clone)]
pub struct MachineAlertHandler {
    pub dispatcher: Arc<NotificationDispatcher<AlertDb>>,
    db_executor: PgPool,
}

impl MachineAlertHandler {
    pub fn new(dispatcher: Arc<NotificationDispatcher<AlertDb>>, db_executor: PgPool) -> Self {
        Self { dispatcher, db_executor }
    }

    pub async fn handle_machine_data_alerts(
        &self,
        pool: &PgPool,
        machine_id: Uuid,
        machine_data: &MachineData,
    ) -> Result<(), MachineAlertError> {
        let organization_id = Machine::get_organization_id(&self.db_executor, machine_id).await?;

        //Alerts derived from the latest machine data
        let extracted_alerts = extract_machine_data_alerts(pool, machine_id, machine_data)
            .await
            .into_iter()
            .map(|alert| NewMachineAlert::new(machine_id, alert))
            .collect::<Vec<_>>();

        // Already known alerts in the database
        let existing_alerts =
            MachineActiveAlert::all_alerts(&self.db_executor, machine_id, organization_id).await?;

        // Alerts that are truly *new* and not duplicates to the alerts already in the database
        let mut filtered_new_alerts =
            self.filter_duplicate_alerts(extracted_alerts.clone(), existing_alerts.clone()).await?;

        // Send notifications for the new alerts
        self.send_notifications(&mut filtered_new_alerts, organization_id as u64, Some(machine_id))
            .await?;

        // Insert the new alerts into the database
        MachineActiveAlert::insert_many(&self.db_executor, &filtered_new_alerts).await?;

        // Resolve step: Remove any alerts that are no longer present
        run_machine_alert_resolution(
            &self.db_executor,
            organization_id,
            extracted_alerts,
            existing_alerts,
        )
        .await?;

        Ok(())
    }
}

#[async_trait]
impl AlertHandler for MachineAlertHandler {
    type Error = MachineAlertError;
    type NewAlertType = NewMachineAlert;
    type ActiveAlertType = MachineActiveAlert;

    fn get_dispatcher(&self) -> &Arc<NotificationDispatcher<AlertDb>> {
        &self.dispatcher
    }

    fn get_db_pool(&self) -> &PgPool {
        &self.db_executor
    }

    async fn filter_duplicate_alerts(
        &self,
        incoming_alerts: Vec<NewMachineAlert>,
        existing_alerts: Vec<MachineActiveAlert>,
    ) -> Result<Vec<NewMachineAlert>, MachineAlertError> {
        let existing_ids = existing_alerts.iter().map(|alert| alert.alert_id).collect::<Vec<_>>();

        let new_filtered_alerts = incoming_alerts
            .into_iter()
            .filter(|alert| !existing_ids.contains(&alert.id))
            .collect::<Vec<_>>();

        Ok(new_filtered_alerts)
    }
}

/// Compare alerts derived from the machine data with the existing alerts in the database.
/// Resolve any alerts that are no longer present in the machine data.
pub async fn run_machine_alert_resolution(
    pool: &PgPool,
    organization_id: i64,
    extracted_alerts: Vec<NewMachineAlert>,
    existing_alerts: Vec<MachineActiveAlert>,
) -> Result<(), MachineAlertError> {
    // Any alert that is present in the existing_alerts but not in the extracted_alerts should be
    // resolved
    for alert in existing_alerts {
        if !extracted_alerts.iter().any(|a| a.id == alert.alert_id) {
            MachineActiveAlert::resolve_alert(pool, alert.alert_id, organization_id).await?;
        }
    }

    Ok(())
}

pub async fn extract_machine_data_alerts(
    pool: &PgPool,
    machine_id: Uuid,
    machine_data: &MachineData,
) -> Vec<Alert> {
    let mut alerts = Vec::new();
    let avs_count = Avs::get_avs_list_count(pool, machine_id).await.unwrap_or(0);
    if avs_count == 0 {
        alerts.push(Alert::IdleMachine { machine_id });
    }

    if machine_data.ivynet_version.is_empty() {
        alerts.push(Alert::ClientUpdateRequired { machine_id });
    }

    let system_metrics = build_system_metrics_from_machine_data(machine_data);
    for error_item in system_metrics.error_items {
        alerts.push(Alert::HardwareResourceUsage {
            machine: machine_id,
            resource: error_item.to_string(),
        });
    }

    todo!()
}

#[cfg(test)]
mod tests {
    use ivynet_notifications::{NotificationConfig, SendgridSpecificTemplates, SendgridTemplates};

    use super::*;

    fn dummy_config_fixture() -> NotificationConfig {
        let specific_templates = SendgridSpecificTemplates {
            custom: "test".to_string(),
            unreg_active_set: "test".to_string(),
            machine_not_responding: "test".to_string(),
            node_not_running: "test".to_string(),
            no_chain_info: "test".to_string(),
            no_metrics: "test".to_string(),
            no_operator: "test".to_string(),
            hw_res_usage: "test".to_string(),
            low_perf: "test".to_string(),
            needs_update: "test".to_string(),
            new_eigen_avs: "test".to_string(),
            updated_eigen_avs: "test".to_string(),
        };

        NotificationConfig {
            telegram_token: "test".to_string(),
            sendgrid_key: "test".to_string(),
            sendgrid_from: "test".to_string(),
            sendgrid_templates: SendgridTemplates::Specific(Box::new(specific_templates)),
        }
    }

    fn handler_fixture(pool: &PgPool) -> MachineAlertHandler {
        MachineAlertHandler::new(
            Arc::new(NotificationDispatcher::new(
                dummy_config_fixture(),
                AlertDb::new(pool.clone()),
            )),
            pool.clone(),
        )
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures(
            "../../../fixtures/new_user_registration.sql",
            "../../../fixtures/machine_alerts_active.sql",
        )
    )]
    #[ignore]
    async fn test_filter_duplicate_alerts(pool: PgPool) {
        let handler = handler_fixture(&pool);
        let machine_id = Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();
        let alert_type_1 = Alert::IdleMachine { machine_id };

        let new_alert_1 = NewMachineAlert::new(machine_id, alert_type_1);

        let alert_type_2 = Alert::IdleMachine { machine_id };
        let new_alert_2 = NewMachineAlert::new(machine_id, alert_type_2.clone());

        let alert_type_3 = Alert::ClientUpdateRequired { machine_id };
        let new_alert_3 = NewMachineAlert::new(machine_id, alert_type_3.clone());

        MachineActiveAlert::insert_one(&pool, &new_alert_1).await.unwrap();

        let alerts = vec![new_alert_2, new_alert_3];

        let existing_alerts = MachineActiveAlert::all_alerts(&pool, machine_id, 1).await.unwrap();

        let filtered_alerts =
            handler.filter_duplicate_alerts(alerts, existing_alerts).await.unwrap();

        assert_eq!(filtered_alerts.len(), 1);
        assert_eq!(filtered_alerts[0].alert_type, alert_type_3);
    }
}
