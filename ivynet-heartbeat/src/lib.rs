use core::fmt;
use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    hash::Hash,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::{DateTime, TimeDelta, Utc};
use ethers::types::Address;
use event::{HeartbeatEvent, HeartbeatEventHandler};
use ivynet_database::error::DatabaseError;
use ivynet_grpc::{
    heartbeat::{
        ClientHeartbeat as ClientHeartbeatSrc, MachineHeartbeat as MachineHeartbeatSrc,
        NodeHeartbeat as NodeHeartbeatSrc,
    },
    tonic::Status,
};
use ivynet_notifications::{
    NotificationDispatcher, NotificationDispatcherError, OrganizationDatabase,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, warn};
use utoipa::ToSchema;
use uuid::Uuid;

mod event;

pub mod alerts;
pub mod server;

const FIFTEEN_MINUTES_SECS: u64 = 15 * 60;
const ONE_MINUTE_SECS: u64 = 60;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientId(Address);

impl TryFrom<ClientHeartbeatSrc> for ClientId {
    type Error = HeartbeatError;

    fn try_from(value: ClientHeartbeatSrc) -> Result<Self, Self::Error> {
        value.client_id.parse().map(ClientId).map_err(|_| HeartbeatError::InvalidClientAddress)
    }
}

impl Display for ClientId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MachineId(Uuid);

impl MachineId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }
}

impl TryFrom<MachineHeartbeatSrc> for MachineId {
    type Error = HeartbeatError;

    fn try_from(value: MachineHeartbeatSrc) -> Result<Self, Self::Error> {
        Uuid::from_str(value.machine_id.as_str())
            .map(MachineId)
            .map_err(|_| HeartbeatError::InvalidMachineAddress)
    }
}

impl Display for MachineId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NodeId {
    pub machine: Uuid,
    pub name: String,
}

impl NodeId {
    pub fn new(machine: Uuid, name: String) -> Self {
        Self { machine, name }
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.machine, self.name)
    }
}

impl TryFrom<NodeHeartbeatSrc> for NodeId {
    type Error = HeartbeatError;

    fn try_from(value: NodeHeartbeatSrc) -> Result<Self, Self::Error> {
        let machine = Uuid::from_str(value.machine_id.as_str())
            .map_err(|_| HeartbeatError::InvalidMachineAddress)?;
        Ok(NodeId { machine, name: value.node_id })
    }
}

impl FromStr for NodeId {
    type Err = HeartbeatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split_once(':');
        match parts {
            Some((machine, name)) => {
                let machine =
                    Uuid::from_str(machine).map_err(|_| HeartbeatError::InvalidMachineAddress)?;
                Ok(NodeId { machine, name: name.to_string() })
            }
            None => Err(HeartbeatError::InvalidNodeAddress),
        }
    }
}

/// A simplified map of heartbeats for a given type to a unix timestamp
pub struct HeartbeatMap<T: Debug + Clone + Eq + Hash + Send + Sync + 'static> {
    map: Arc<RwLock<HashMap<T, DateTime<Utc>>>>,
}

impl<T: Debug + Eq + Hash + Send + Sync + Clone + 'static> Default for HeartbeatMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug + Eq + Hash + Send + Sync + Clone + 'static> HeartbeatMap<T> {
    pub fn new() -> Self {
        let map: Arc<RwLock<HashMap<T, DateTime<Utc>>>> = Arc::new(RwLock::new(HashMap::new()));
        Self { map }
    }

    pub fn insert(&self, key: T) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        self.map.write().expect("Write lock failed").insert(key, now)
    }

    pub fn remove(&self, key: &T) -> Option<DateTime<Utc>> {
        self.map.write().expect("Write lock failed").remove(key)
    }

    pub fn get(&self, key: &T) -> Option<DateTime<Utc>> {
        self.map.read().expect("Read lock failed").get(key).copied()
    }

    pub fn get_all(&self) -> HashMap<T, DateTime<Utc>> {
        self.map.read().expect("Read lock failed").clone()
    }

    pub fn remove_stale_entries(&self, ttl: TimeDelta) -> Vec<(T, DateTime<Utc>)> {
        let now = Utc::now();
        let mut map = self.map.write().expect("Write lock failed");
        let stale_entries = map
            .iter()
            .filter(|(_, &time)| now - time > ttl)
            .map(|(k, v)| (k.clone(), *v))
            .collect::<Vec<_>>();

        for (key, _) in &stale_entries {
            map.remove(key);
        }

        stale_entries
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HeartbeatError {
    #[error("invalid client address")]
    InvalidClientAddress,
    #[error("invalid machine address")]
    InvalidMachineAddress,
    #[error("invalid node address")]
    InvalidNodeAddress,
    #[error("database error: {0}")]
    DatabaseError(#[from] DatabaseError),
    #[error("notification error: {0}")]
    NotificationError(#[from] NotificationDispatcherError),
}

impl From<HeartbeatError> for Status {
    fn from(err: HeartbeatError) -> Self {
        Status::invalid_argument(err.to_string())
    }
}

pub struct HeartbeatMonitor<D: OrganizationDatabase> {
    client_map: Arc<HeartbeatMap<ClientId>>,
    machine_map: Arc<HeartbeatMap<MachineId>>,
    node_map: Arc<HeartbeatMap<NodeId>>,
    event_handler: Arc<HeartbeatEventHandler<D>>,
}

impl<D: OrganizationDatabase> HeartbeatMonitor<D> {
    pub fn new(db: PgPool, notifier: Arc<NotificationDispatcher<D>>) -> Self {
        let client_map = Arc::new(HeartbeatMap::new());
        let machine_map = Arc::new(HeartbeatMap::new());
        let node_map = Arc::new(HeartbeatMap::new());
        let event_handler = Arc::new(HeartbeatEventHandler::new(db, notifier));

        let monitor = Self { client_map, machine_map, node_map, event_handler };

        let client_map = Arc::clone(&monitor.client_map);
        let machine_map = Arc::clone(&monitor.machine_map);
        let node_map = Arc::clone(&monitor.node_map);
        let event_handler = Arc::clone(&monitor.event_handler);

        tokio::spawn(async move {
            let interval = Duration::from_secs(ONE_MINUTE_SECS);
            let ttl = TimeDelta::seconds(FIFTEEN_MINUTES_SECS as i64);
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                let stale_clients = client_map.remove_stale_entries(ttl);
                let stale_machines = machine_map.remove_stale_entries(ttl);
                let stale_nodes = node_map.remove_stale_entries(ttl);

                for (client_id, time) in stale_clients {
                    let event = HeartbeatEvent::StaleClient { client_id, last_heartbeat: time };
                    if let Err(e) = event_handler.handle_event(event).await {
                        error!("Error handling stale client event: {}", e);
                    };
                }

                for (machine_id, time) in stale_machines {
                    let event = HeartbeatEvent::StaleMachine { machine_id, last_heartbeat: time };
                    if let Err(e) = event_handler.handle_event(event).await {
                        error!("Error handling stale machine event: {}", e);
                    };
                }

                for (node_id, time) in stale_nodes {
                    let event = HeartbeatEvent::StaleNode { node_id, last_heartbeat: time };
                    if let Err(e) = event_handler.handle_event(event).await {
                        error!("Error handling stale node event: {}", e);
                    };
                }
            }
        });

        monitor
    }

    pub async fn post_client_heartbeat(&self, client_id: ClientId) -> Result<(), HeartbeatError> {
        if self.client_map.insert(client_id).is_none() {
            let event = HeartbeatEvent::NewClient(client_id);
            self.event_handler.handle_event(event).await?;
        }
        Ok(())
    }

    pub async fn post_machine_heartbeat(
        &self,
        machine_id: MachineId,
    ) -> Result<(), HeartbeatError> {
        if self.machine_map.insert(machine_id).is_none() {
            let event = HeartbeatEvent::NewMachine(machine_id);
            self.event_handler.handle_event(event).await?;
        }
        Ok(())
    }

    pub async fn post_node_heartbeat(&self, node_id: NodeId) -> Result<(), HeartbeatError> {
        if self.node_map.insert(node_id.clone()).is_none() {
            let event = HeartbeatEvent::NewNode(node_id);
            self.event_handler.handle_event(event).await?;
        }
        Ok(())
    }
}

impl<D: OrganizationDatabase> Drop for HeartbeatMonitor<D> {
    fn drop(&mut self) {
        warn!("Shutting down heartbeat monitor");
    }
}
