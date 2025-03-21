use async_trait::async_trait;
use ivynet_alerts::{Alert, SendState};
use ivynet_notifications::{
    Channel, Notification, NotificationDispatcher, NotificationDispatcherError,
};
use std::sync::Arc;

use sqlx::{types::Uuid, PgPool};

use super::alert_db::AlertDb;
use crate::NotificationSettings;

/// Represents a new alert that can be created for either nodes or organizations
pub trait NewAlert {
    fn get_id(&self) -> Uuid;
    fn get_alert_type(&self) -> Alert;
    fn set_send_state(&mut self, channel: &Channel, state: SendState);
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

    /// Send notifications for the given alerts through configured channels
    async fn send_notifications(
        &self,
        alerts: &mut Vec<Self::NewAlertType>,
        organization_id: u64,
        machine_id: Option<Uuid>,
    ) -> Result<(), Self::Error> {
        let settings = NotificationSettings::get(self.get_db_pool(), organization_id)
            .await
            .expect("Organization notifications not found");
        let enabled_alert_ids = settings.alert_flags.to_alert_ids();

        let mut channels = Vec::new();
        if settings.pagerduty {
            channels.push(Channel::PagerDuty(settings.pagerduty_keys));
        }
        if settings.email {
            channels.push(Channel::Email(settings.sendgrid_emails));
        }
        if settings.telegram {
            channels.push(Channel::Telegram(settings.telegram_chats));
        }

        for alert in alerts.iter_mut() {
            for channel in channels.iter() {
                if enabled_alert_ids.contains(&alert.get_alert_type().id()) {
                    let notification = Notification {
                        id: alert.get_id(),
                        organization: organization_id,
                        machine_id,
                        alert: alert.get_alert_type(),
                        resolved: false,
                    };

                    let dispatcher = self.get_dispatcher();

                    let send_state = match dispatcher.notify_channel(notification, channel).await {
                        true => SendState::SendSuccess,
                        false => SendState::SendFailed,
                    };

                    println!("send_state: {:#?}", send_state);

                    alert.set_send_state(channel, send_state);
                }
            }
        }

        Ok(())
    }
}
