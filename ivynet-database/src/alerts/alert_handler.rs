use async_trait::async_trait;
use ivynet_alerts::{Alert, Channel, SendState};
use ivynet_notifications::{Notification, NotificationDispatcher, NotificationDispatcherError};
use std::{collections::HashMap, sync::Arc};

use sqlx::{types::Uuid, PgPool};

use super::alert_db::AlertDb;
use crate::NotificationSettings;

/// Represents a new alert that can be created for either nodes or organizations
pub trait NewAlert {
    fn get_id(&self) -> Uuid;
    fn get_alert_type(&self) -> Alert;
    fn set_send_state(&mut self, channel: Channel, state: SendState);
}

/// Represents an active alert that can be retrieved from the database
pub trait ActiveAlert {
    fn get_id(&self) -> Uuid;
    fn get_alert_type(&self) -> Alert;
}

/// Common trait for alert handlers that provides shared functionality
#[async_trait]
pub trait AlertHandler {
    type Error: From<NotificationDispatcherError>;
    type NewAlertType: NewAlert + Send;
    type ActiveAlertType: ActiveAlert + Send;

    fn get_dispatcher(&self) -> &Arc<NotificationDispatcher<AlertDb>>;
    fn get_db_pool(&self) -> &PgPool;

    /// Filter out duplicate alerts that already exist in the database
    async fn filter_duplicate_alerts(
        &self,
        incoming_alerts: Vec<Self::NewAlertType>,
        existing_alerts: Vec<Self::ActiveAlertType>,
    ) -> Result<Vec<Self::NewAlertType>, Self::Error>;

    /// Get the notification channels and alert flags for an organization
    /// Returns hashmap of organization enabled / disabled notification chanels, as well as flags
    /// for enabled/disabled alerts in the form of a vec.
    async fn organization_channel_alerts(
        &self,
        organization_id: u64,
    ) -> (HashMap<Channel, bool>, Vec<usize>) {
        let mut channels = HashMap::new();
        let org_notifications = NotificationSettings::get(self.get_db_pool(), organization_id)
            .await
            .expect("Organization notifications not found");

        channels.insert(Channel::Telegram, org_notifications.telegram);
        channels.insert(Channel::Email, org_notifications.email);
        channels.insert(Channel::PagerDuty, org_notifications.pagerduty);

        (channels, org_notifications.alert_flags.to_alert_ids())
    }

    /// Send notifications for the given alerts through configured channels
    async fn send_notifications(
        &self,
        alerts: &mut Vec<Self::NewAlertType>,
        organization_id: u64,
        machine_id: Option<Uuid>,
    ) -> Result<(), Self::Error> {
        let (channels, enabled_alert_ids) = self.organization_channel_alerts(organization_id).await;

        for (channel, do_send) in channels {
            for alert in alerts.iter_mut() {
                if do_send && enabled_alert_ids.contains(&alert.get_alert_type().id()) {
                    let notification = Notification {
                        id: alert.get_id(),
                        organization: organization_id,
                        machine_id,
                        alert: alert.get_alert_type(),
                        resolved: false,
                    };

                    let send_state =
                        match self.get_dispatcher().notify_channel(notification, channel).await {
                            true => SendState::SendSuccess,
                            false => SendState::SendFailed,
                        };
                    alert.set_send_state(channel, send_state);
                }
            }
        }

        Ok(())
    }
}
