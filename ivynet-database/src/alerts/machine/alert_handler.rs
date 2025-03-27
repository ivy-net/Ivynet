use std::sync::Arc;

use ivynet_alerts::{Alert, SendState};
use ivynet_notifications::{Channel, NotificationDispatcher, NotificationDispatcherError};
use sqlx::{types::Uuid, PgPool};

use async_trait::async_trait;

use crate::{
    alerts::{
        alert_db::AlertDb,
        alert_handler::{ActiveAlert, AlertHandler, NewAlert},
    },
    error::DatabaseError,
    Machine,
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

    pub async fn handle_machine_alerts(
        &self,
        machine_id: Uuid,
        alerts: Vec<Alert>,
    ) -> Result<(), MachineAlertError> {
        let organization_id = Machine::get_organization_id(&self.db_executor, machine_id).await?;

        let new_alerts = alerts
            .into_iter()
            .map(|alert| NewMachineAlert::new(machine_id, alert))
            .collect::<Vec<_>>();

        let existing_alerts = MachineActiveAlert::all_alerts_by_machine(
            &self.db_executor,
            machine_id,
            organization_id,
        )
        .await?;

        let mut filtered_new_alerts =
            self.filter_duplicate_alerts(new_alerts, existing_alerts).await?;

        self.send_notifications(&mut filtered_new_alerts, organization_id as u64, Some(machine_id))
            .await?;

        MachineActiveAlert::insert_many(&self.db_executor, &filtered_new_alerts).await?;

        // Resolve step
        run_machine_alert_resolution(&self.db_executor, machine_id, organization_id).await?;

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

        let filtered = incoming_alerts
            .into_iter()
            .filter(|alert| !existing_ids.contains(&alert.id))
            .collect::<Vec<_>>();

        Ok(filtered)
    }
}

/// Compare alerts derived from the machine data with the existing alerts in the database.
/// Resolve any alerts that are no longer present in the machine data.
pub async fn run_machine_alert_resolution(
    pool: &PgPool,
    machine_id: Uuid,
    organization_id: i64,
) -> Result<(), MachineAlertError> {
    let alerts =
        MachineActiveAlert::all_alerts_by_machine(pool, machine_id, organization_id).await?;

    // Filter existing alerts, removing any that are not in the incoming list
    let to_resolve = alerts
        .into_iter()
        .filter(|alert| {
            // FIXME: Add logic to determine which alerts should be resolved
            // For now, we'll just resolve all alerts
            true
        })
        .collect::<Vec<_>>();

    for alert in to_resolve {
        MachineActiveAlert::resolve_alert(pool, alert.alert_id, organization_id).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use ivynet_node_type::NodeType;
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
    async fn test_filter_duplicate_alerts(pool: PgPool) {
        let handler = handler_fixture(&pool);
        let machine_id = Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();

        // Create a new alert that matches one in the fixture
        let alert_type_1 = Alert::Custom {
            node_name: "test_machine".to_string(),
            node_type: NodeType::EigenDA.to_string(),
            extra_data: serde_json::Value::String("runtime_alert_fixture_1".to_string()),
        };
        let new_alert_1 = NewMachineAlert::new(machine_id, alert_type_1);

        // Create a new alert that doesn't exist in the fixture
        let alert_type_2 = Alert::IdleMachine { machine_id };
        let new_alert_2 = NewMachineAlert::new(machine_id, alert_type_2.clone());

        let alerts = vec![new_alert_1, new_alert_2];

        // Get existing alerts from the fixture
        let existing_alerts =
            MachineActiveAlert::all_alerts_by_machine(&pool, machine_id, 1).await.unwrap();

        // Filter out duplicates
        let filtered_alerts =
            handler.filter_duplicate_alerts(alerts, existing_alerts).await.unwrap();

        // Should only contain the new alert that doesn't exist in the fixture
        assert_eq!(filtered_alerts.len(), 1);
        assert_eq!(filtered_alerts[0].alert_type, alert_type_2);
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures(
            "../../../fixtures/new_user_registration.sql",
            "../../../fixtures/machine_alerts_active.sql",
        )
    )]
    async fn test_handle_machine_alerts(pool: PgPool) {
        let handler = handler_fixture(&pool);
        let machine_id = Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();

        // Create a new alert
        let alert = Alert::IdleMachine { machine_id };
        let alerts = vec![alert];

        // Handle the alerts
        handler.handle_machine_alerts(machine_id, alerts).await.unwrap();

        // Verify the alert was inserted
        let existing_alerts =
            MachineActiveAlert::all_alerts_by_machine(&pool, machine_id, 1).await.unwrap();
        assert!(existing_alerts
            .iter()
            .any(|a| matches!(a.alert_type, Alert::IdleMachine { machine_id: _ })));
    }
}
