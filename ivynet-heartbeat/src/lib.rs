use core::fmt;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    hash::Hash,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
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
use tokio::task::JoinHandle;
use tracing::debug;
use uuid::Uuid;

mod event;

pub mod alerts;
pub mod server;

const FIVE_MINUTES: TimeDelta = TimeDelta::minutes(5);
const ONE_MINUTE: TimeDelta = TimeDelta::minutes(1);

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

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
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

// Add a type alias for the callback function to make the code more readable
type StaleCallback<T> = dyn Fn(T, DateTime<Utc>) + Send + Sync + 'static;

/// A map of heartbeats for a given type to a unix timestamp
pub struct HeartbeatMap<T: Clone + Eq + Hash + Send + Sync + 'static> {
    _ttl: TimeDelta,
    map: Arc<RwLock<HashMap<T, DateTime<Utc>>>>,
    cleanup_thread: Option<JoinHandle<()>>,
    stop_signal: Arc<AtomicBool>,
    _on_stale_callback: Option<Arc<StaleCallback<T>>>,
}

impl<T: Eq + Hash + Send + Sync + Clone + 'static> Default for HeartbeatMap<T> {
    fn default() -> Self {
        Self::new(Duration::from_secs(60), FIVE_MINUTES, None::<fn(T, DateTime<Utc>)>)
    }
}

impl<T: Eq + Hash + Send + Sync + Clone + 'static> HeartbeatMap<T> {
    pub fn new(
        cleanup_interval: Duration,
        ttl: TimeDelta,
        on_stale_callback: Option<impl Fn(T, DateTime<Utc>) + Send + Sync + 'static>,
    ) -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();

        let map: Arc<RwLock<HashMap<T, DateTime<Utc>>>> = Arc::new(RwLock::new(HashMap::new()));
        let map_clone = map.clone();

        let callback = on_stale_callback.map(|cb| Arc::new(cb) as Arc<StaleCallback<T>>);
        let callback_clone = callback.clone();

        let cleanup_thread = Some(tokio::spawn(async move {
            loop {
                tokio::time::sleep(cleanup_interval).await;
                if stop_signal_clone.load(Ordering::Relaxed) {
                    break;
                }
                let now = Utc::now();
                let mut map = map_clone.write().expect("Write lock failed");
                let stale_entries = map
                    .iter()
                    .filter(|(_, &time)| now - time > ttl)
                    .map(|(k, v)| (k.clone(), *v))
                    .collect::<Vec<_>>();

                for (key, time) in stale_entries {
                    map.remove(&key);
                    if let Some(ref callback) = callback_clone {
                        callback(key, time);
                    }
                }
            }
        }));
        Self { _ttl: ttl, map, cleanup_thread, stop_signal, _on_stale_callback: callback }
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

    pub fn shutdown(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}

impl<T: Eq + Hash + Send + Sync + Clone + 'static> Drop for HeartbeatMap<T> {
    fn drop(&mut self) {
        // Signal the cleanup thread to stop
        self.stop_signal.store(true, Ordering::Relaxed);

        // Optionally, if you want to wait for the thread to finish:
        if let Some(handle) = self.cleanup_thread.take() {
            // Convert to a blocking operation only in drop
            handle.abort();
        }
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
    client_map: HeartbeatMap<ClientId>,
    machine_map: HeartbeatMap<MachineId>,
    node_map: HeartbeatMap<NodeId>,
    event_handler: Arc<HeartbeatEventHandler<D>>,
}

impl<D: OrganizationDatabase> HeartbeatMonitor<D> {
    pub fn new(db: PgPool, notifier: Arc<NotificationDispatcher<D>>) -> Self {
        let event_handler = HeartbeatEventHandler::new(db, notifier);
        let event_handler_arc = Arc::new(event_handler);
        let event_handler_client = event_handler_arc.clone();
        let event_handler_machine = event_handler_arc.clone();
        let event_handler_node = event_handler_arc.clone();

        // Create callback for client heartbeats
        let client_callback = move |client_id: ClientId, last_heartbeat: DateTime<Utc>| {
            let handler = event_handler_client.clone();
            tokio::spawn(async move {
                let event = HeartbeatEvent::StaleClient { client_id, last_heartbeat };
                if let Err(e) = handler.handle_event(event).await {
                    eprintln!("Error handling stale client event: {}", e);
                }
            });
        };

        // Create callback for machine heartbeats
        let machine_callback = move |machine_id: MachineId, time_not_responding: DateTime<Utc>| {
            let handler = event_handler_machine.clone();
            tokio::spawn(async move {
                let event = HeartbeatEvent::StaleMachine { machine_id, time_not_responding };
                debug!("Stale machine heartbeat: {:?}", event);
                if let Err(e) = handler.handle_event(event).await {
                    eprintln!("Error handling stale machine event: {}", e);
                }
            });
        };

        // Create callback for node heartbeats
        let node_callback = move |node_id: NodeId, time_not_responding: DateTime<Utc>| {
            let handler = event_handler_node.clone();
            tokio::spawn(async move {
                let event = HeartbeatEvent::StaleNode { node_id, time_not_responding };
                debug!("Stale node heartbeat: {:?}", event);
                if let Err(e) = handler.handle_event(event).await {
                    eprintln!("Error handling stale node event: {}", e);
                }
            });
        };

        let client_map =
            HeartbeatMap::new(Duration::from_secs(60), ONE_MINUTE, Some(client_callback));
        let machine_map =
            HeartbeatMap::new(Duration::from_secs(60), ONE_MINUTE, Some(machine_callback));
        let node_map = HeartbeatMap::new(Duration::from_secs(60), ONE_MINUTE, Some(node_callback));

        Self { client_map, machine_map, node_map, event_handler: event_handler_arc }
    }

    pub async fn post_client_heartbeat(&self, client_id: ClientId) -> Result<(), HeartbeatError> {
        debug!("Client heartbeat: {}", client_id.to_string());
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
        debug!("Machine heartbeat: {}", machine_id.to_string());
        if self.machine_map.insert(machine_id).is_none() {
            let event = HeartbeatEvent::NewMachine(machine_id);
            self.event_handler.handle_event(event).await?;
        }
        Ok(())
    }

    pub async fn post_node_heartbeat(&self, node_id: NodeId) -> Result<(), HeartbeatError> {
        debug!("Node heartbeat: {}", node_id.to_string());
        if self.node_map.insert(node_id.clone()).is_none() {
            let event = HeartbeatEvent::NewNode(node_id);
            self.event_handler.handle_event(event).await?;
        }
        Ok(())
    }
}
