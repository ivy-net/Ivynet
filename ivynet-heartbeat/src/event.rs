use std::sync::Arc;

use chrono::{DateTime, Utc};
use ivynet_alerts::AlertType;
use ivynet_database::{error::DatabaseError, NotificationSettings};
use ivynet_notifications::{NotificationDispatcher, OrganizationDatabase};
use sqlx::PgPool;

use crate::{
    alerts::{ClientHeartbeatAlert, MachineHeartbeatAlert, NodeHeartbeatAlert},
    ClientId, HeartbeatError, MachineId, NodeId,
};

#[derive(Debug)]
pub enum HeartbeatEvent {
    NewClient(ClientId),
    NewMachine(MachineId),
    NewNode(NodeId),
    StaleClient { client_id: ClientId, last_heartbeat: DateTime<Utc> },
    StaleMachine { machine_id: MachineId, last_heartbeat: DateTime<Utc> },
    StaleNode { node_id: NodeId, last_heartbeat: DateTime<Utc> },
}

pub struct HeartbeatEventHandler<D: OrganizationDatabase> {
    pub db: PgPool,
    pub notifier: Arc<NotificationDispatcher<D>>,
}

impl<D: OrganizationDatabase> HeartbeatEventHandler<D> {
    pub fn new(db: PgPool, notifier: Arc<NotificationDispatcher<D>>) -> Self {
        Self { db, notifier }
    }

    /// Top-level event handler that delegates to specialized methods.
    pub async fn handle_event(&self, event: HeartbeatEvent) -> Result<(), HeartbeatError> {
        match event {
            HeartbeatEvent::NewClient(client_id) => self.handle_new_client(client_id).await?,
            HeartbeatEvent::NewMachine(machine_id) => self.handle_new_machine(machine_id).await?,
            HeartbeatEvent::NewNode(node_id) => self.handle_new_node(node_id).await?,
            HeartbeatEvent::StaleClient { client_id, last_heartbeat } => {
                let settings = NotificationSettings::get_for_client(&self.db, client_id.0).await?;
                self.handle_stale_client(
                    client_id,
                    settings.organization_id,
                    last_heartbeat,
                    settings,
                )
                .await?
            }
            HeartbeatEvent::StaleMachine { machine_id, last_heartbeat } => {
                let settings =
                    NotificationSettings::get_for_machine(&self.db, machine_id.0).await?;
                self.handle_stale_machine(
                    machine_id,
                    settings.organization_id,
                    last_heartbeat,
                    settings,
                )
                .await?
            }
            HeartbeatEvent::StaleNode { node_id, last_heartbeat } => {
                let settings =
                    NotificationSettings::get_for_machine(&self.db, node_id.machine).await?;
                self.handle_stale_node(node_id, settings.organization_id, last_heartbeat, settings)
                    .await?
            }
        }
        Ok(())
    }

    async fn handle_new_client(&self, client_id: ClientId) -> Result<(), DatabaseError> {
        ClientHeartbeatAlert::resolve(&self.db, client_id).await?;
        Ok(())
    }

    async fn handle_new_machine(&self, machine_id: MachineId) -> Result<(), DatabaseError> {
        MachineHeartbeatAlert::resolve(&self.db, machine_id).await?;
        Ok(())
    }

    async fn handle_new_node(&self, node_id: NodeId) -> Result<(), DatabaseError> {
        NodeHeartbeatAlert::resolve(&self.db, node_id).await?;
        Ok(())
    }

    async fn handle_stale_client(
        &self,
        client_id: ClientId,
        organization_id: i64,
        last_response_time: DateTime<Utc>,
        settings: NotificationSettings,
    ) -> Result<(), HeartbeatError> {
        let alert = ClientHeartbeatAlert { client_id, last_response_time, created_at: Utc::now() };
        ClientHeartbeatAlert::insert(&self.db, alert.clone(), organization_id).await?;
        if settings
            .alert_flags
            .is_alert_enabled(&AlertType::NoMachineHeartbeat)
            .is_ok_and(|enabled| enabled)
        {
            let channels = settings.get_active_channels();
            self.notifier.notify(alert, channels).await?;
        }
        Ok(())
    }

    async fn handle_stale_machine(
        &self,
        machine_id: MachineId,
        organization_id: i64,
        last_response_time: DateTime<Utc>,
        settings: NotificationSettings,
    ) -> Result<(), HeartbeatError> {
        let alert =
            MachineHeartbeatAlert { machine_id, last_response_time, created_at: Utc::now() };
        MachineHeartbeatAlert::insert(&self.db, alert.clone(), organization_id).await?;
        if settings
            .alert_flags
            .is_alert_enabled(&AlertType::NoMachineHeartbeat)
            .is_ok_and(|enabled| enabled)
        {
            let channels = settings.get_active_channels();
            self.notifier.notify(alert, channels).await?;
        }

        Ok(())
    }

    async fn handle_stale_node(
        &self,
        node_id: NodeId,
        organization_id: i64,
        last_response_time: DateTime<Utc>,
        settings: NotificationSettings,
    ) -> Result<(), HeartbeatError> {
        let alert = NodeHeartbeatAlert { node_id, last_response_time, created_at: Utc::now() };
        NodeHeartbeatAlert::insert(&self.db, alert.clone(), organization_id).await?;
        if settings
            .alert_flags
            .is_alert_enabled(&AlertType::NoMachineHeartbeat)
            .is_ok_and(|enabled| enabled)
        {
            let channels = settings.get_active_channels();
            self.notifier.notify(alert, channels).await?;
        }
        Ok(())
    }
}
