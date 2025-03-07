use async_trait::async_trait;
use ivynet_alerts::Alert;
use ivynet_notifications::{
    Channel, Notification, NotificationDispatcher, NotificationDispatcherError,
};
use sqlx::{types::Uuid, PgPool};
use std::{collections::HashSet, sync::Arc};

use super::alert_db::AlertDb;
use crate::NotificationSettings;

/// Represents a new alert that can be created for either nodes or organizations
pub trait NewAlert {
    fn get_id(&self) -> Uuid;
    fn get_alert_type(&self) -> Alert;
}

/// Common trait for alert handlers that provides shared functionality
#[async_trait]
pub trait AlertHandler {
    type Error: From<NotificationDispatcherError>;
    type AlertType: NewAlert + Send;

    fn get_dispatcher(&self) -> &Arc<NotificationDispatcher<AlertDb>>;
    fn get_db_pool(&self) -> &PgPool;

    /// Filter out duplicate alerts that already exist in the database
    async fn filter_duplicate_alerts(
        &self,
        alerts: Vec<Self::AlertType>,
    ) -> Result<Vec<Self::AlertType>, Self::Error>;

    /// Get the notification channels and alert flags for an organization
    async fn organization_channel_alerts(
        &self,
        organization_id: u64,
    ) -> (HashSet<Channel>, Vec<usize>) {
        let mut channels = HashSet::new();
        let org_notifications = NotificationSettings::get(self.get_db_pool(), organization_id)
            .await
            .expect("Organization notifications not found");

        if org_notifications.telegram {
            channels.insert(Channel::Telegram);
        }
        if org_notifications.email {
            channels.insert(Channel::Email);
        }
        if org_notifications.pagerduty {
            channels.insert(Channel::PagerDuty);
        }

        (channels, org_notifications.alert_flags.to_alert_ids())
    }

    /// Send notifications for the given alerts through configured channels
    async fn send_notifications(
        &self,
        alerts: Vec<Self::AlertType>,
        organization_id: u64,
        machine_id: Option<Uuid>,
    ) -> Result<(), Self::Error> {
        let (channels, alert_ids) = self.organization_channel_alerts(organization_id).await;

        let notifications: Vec<Notification> = alerts
            .into_iter()
            .filter(|alert| alert_ids.contains(&alert.get_alert_type().id()))
            .map(|alert| Notification {
                id: alert.get_id(),
                organization: organization_id,
                machine_id,
                alert: alert.get_alert_type(),
                resolved: false,
            })
            .collect();

        for notification in notifications {
            self.get_dispatcher().notify(notification, channels.clone()).await?;
        }

        Ok(())
    }
}
