use chrono::{DateTime, Utc};

use crate::{ClientId, MachineId, NodeId};

pub struct ClientHeartbeatAlert {
    pub client_id: ClientId,
    pub last_response_time: String,
}

pub struct MachineHeartbeatAlert {
    pub machine_id: MachineId,
    pub last_response_time: String,
}

pub struct NodeHeartbeatAlert {
    pub node_id: NodeId,
    pub last_response_time: String,
}
