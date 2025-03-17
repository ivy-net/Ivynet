use chrono::{DateTime, Utc};
use ivynet_grpc::heartbeat::{ClientHeartbeat, MachineHeartbeat, NodeHeartbeat};
use sqlx::PgPool;

use crate::{
    alert::{ClientHeartbeatAlert, MachineHeartbeatAlert, NodeHeartbeatAlert},
    ClientId, MachineId, NodeId,
};

pub enum HeartbeatEvent {
    NewClient(ClientHeartbeat),
    NewMachine(MachineHeartbeat),
    NewNode(NodeHeartbeat),
    StaleClient { client_id: ClientId, last_heartbeat: DateTime<Utc> },
    StaleMachine { machine_id: MachineId, time_not_responding: DateTime<Utc> },
    StaleNode { node_id: NodeId, time_not_responding: DateTime<Utc> },
}

pub struct HeartbeatEventHandler {
    pub db: PgPool,
}

impl HeartbeatEventHandler {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn handle_event(&self, event: HeartbeatEvent) {
        match event {
            HeartbeatEvent::NewClient(client_heartbeat) => {
                let alert = ClientHeartbeatAlert {
                    client_id: client_heartbeat.client_id.to_string(),
                    last_response_time: Utc::now(),
                };
                sqlx::query!(
                    r#"
                    INSERT INTO client_heartbeat_alerts (client_id, last_response_time)
                    VALUES ($1, $2)
                    "#,
                    alert.client_id,
                    alert.last_response_time,
                )
            }
            HeartbeatEvent::NewMachine(machine_heartbeat) => {
                let alert = MachineHeartbeatAlert {
                    machine_id: machine_heartbeat.machine_id.to_string(),
                    last_response_time: Utc::now(),
                };
                sqlx::query!(
                    r#"
                    INSERT INTO machine_heartbeat_alerts (machine_id, last_response_time)
                    VALUES ($1, $2)
                    "#,
                    alert.machine_id,
                    alert.last_response_time,
                )
            }
            HeartbeatEvent::NewNode(node_heartbeat) => {
                let alert = NodeHeartbeatAlert {
                    node_id: node_heartbeat.node_id.to_string(),
                    last_response_time: Utc::now(),
                };
                sqlx::query!(
                    r#"
                    INSERT INTO node_heartbeat_alerts (node_id, last_response_time)
                    VALUES ($1, $2)
                    "#,
                    alert.node_id,
                    alert.last_response_time,
                )
            }
            HeartbeatEvent::StaleClient { client_id, last_heartbeat } => {
                sqlx_query!(
                    r#"
                    DELETE FROM client_heartbeat_alerts
                    WHERE client_id = $1
                    "#,
                    client_id,
                )
            }
            HeartbeatEvent::StaleMachine { machine_id, time_not_responding } => {
                sqlx_query!(
                    r#"
                    DELETE FROM machine_heartbeat_alerts
                    WHERE machine_id = $1
                    "#,
                    machine_id,
                )
            }
            HeartbeatEvent::StaleNode { node_id, time_not_responding } => {
                sqlx_query!(
                    r#"
                    DELETE FROM node_heartbeat_alerts
                    WHERE node_id = $1
                    "#,
                    node_id,
                )
            }
        }
    }
}
