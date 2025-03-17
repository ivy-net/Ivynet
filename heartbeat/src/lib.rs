use core::fmt;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use ethers::types::Address;
use ivynet_grpc::{
    heartbeat::{
        heartbeat_server::{Heartbeat, HeartbeatServer},
        ClientHeartbeat as ClientHeartbeatSrc, MachineHeartbeat as MachineHeartbeatSrc,
        NodeHeartbeat as NodeHeartbeatSrc,
    },
    server::{Endpoint, Server},
    tonic::{Request, Response, Status},
};
use tokio::task::JoinHandle;
use uuid::Uuid;

mod alert;
mod event;

const FIVE_MINUTES: TimeDelta = TimeDelta::minutes(5);

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
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

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MachineId(Uuid);

impl TryFrom<MachineHeartbeatSrc> for MachineId {
    type Error = HeartbeatError;

    fn try_from(value: MachineHeartbeatSrc) -> Result<Self, Self::Error> {
        Uuid::from_str(value.machine_id.as_str())
            .map(MachineId)
            .map_err(|_| HeartbeatError::InvalidMachineAddress)
    }
}

impl Dsiplay for MachineId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct NodeId {
    pub machine: Uuid,
    pub name: String,
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

/// A map of heartbeats for a given type to a unix timestamp
pub struct HeartbeatMap<T: Eq + Hash + Send + Sync + 'static> {
    ttl: TimeDelta,
    map: Arc<RwLock<HashMap<T, DateTime<Utc>>>>,
    cleanup_thread: Option<JoinHandle<()>>,
    stop_signal: Arc<AtomicBool>,
}

impl<T: Eq + Hash + Send + Sync + 'static> Default for HeartbeatMap<T> {
    fn default() -> Self {
        Self::new(Duration::from_secs(60), FIVE_MINUTES)
    }
}

impl<T: Eq + Hash + Send + Sync + 'static> HeartbeatMap<T> {
    pub fn new(cleanup_interval: Duration, ttl: TimeDelta) -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();

        let map: Arc<RwLock<HashMap<T, DateTime<Utc>>>> = Arc::new(RwLock::new(HashMap::new()));
        let map_clone = map.clone();

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
                    .map(|(k, _)| k.clone())
                    .collect::<Vec<_>>();
            }
        }));
        Self { ttl, map, cleanup_thread, stop_signal }
    }

    pub fn insert(&self, key: T) {
        let now = Utc::now();
        self.map.write().expect("Write lock failed").insert(key, now);
    }

    pub fn remove(&self, key: &T) {
        self.map.write().expect("Write lock failed").remove(key);
    }

    pub fn get(&self, key: &T) -> Option<DateTime<Utc>> {
        self.map.read().expect("Read lock failed").get(key).copied()
    }

    pub fn shutdown(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
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
}

impl From<HeartbeatError> for Status {
    fn from(err: HeartbeatError) -> Self {
        Status::invalid_argument(err.to_string())
    }
}

pub struct HeartbeatService {
    client_map: HeartbeatMap<ClientId>,
    machine_map: HeartbeatMap<MachineId>,
    node_map: HeartbeatMap<NodeId>,
}

impl HeartbeatService {
    pub fn new() -> Self {
        let client_map = HeartbeatMap::default();
        let machine_map = HeartbeatMap::default();
        let node_map = HeartbeatMap::default();
        Self { client_map, machine_map, node_map }
    }
}

impl Default for HeartbeatService {
    fn default() -> Self {
        Self::new()
    }
}

#[ivynet_grpc::async_trait]
impl Heartbeat for HeartbeatService {
    async fn send_client_heartbeat(
        &self,
        request: Request<ClientHeartbeatSrc>,
    ) -> Result<Response<()>, Status> {
        let client_id = request.into_inner().try_into()?;
        self.client_map.insert(client_id);
        Ok(Response::new(()))
    }

    async fn send_machine_heartbeat(
        &self,
        request: Request<MachineHeartbeatSrc>,
    ) -> Result<Response<()>, Status> {
        let machine_id = request.into_inner().try_into()?;
        self.machine_map.insert(machine_id);
        Ok(Response::new(()))
    }

    async fn send_node_heartbeat(
        &self,
        request: Request<NodeHeartbeatSrc>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let node_id = req.try_into()?;
        self.node_map.insert(node_id);
        Ok(Response::new(()))
    }
}

async fn serve(
    tls_cert: Option<String>,
    tls_key: Option<String>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::new(HeartbeatServer::new(HeartbeatService::new()), tls_cert, tls_key);
    let endpoint = Endpoint::Port(port);
    server.serve(endpoint).await?;
    Ok(())
}
