use std::sync::Arc;

use ivynet_database::alerts::alert_db::AlertDb;
use ivynet_grpc::{
    heartbeat::{
        heartbeat_server::{Heartbeat, HeartbeatServer},
        ClientHeartbeat as ClientHeartbeatSrc, MachineHeartbeat as MachineHeartbeatSrc,
        NodeHeartbeat as NodeHeartbeatSrc,
    },
    server::{Endpoint, Server},
    tonic::{Request, Response, Status},
};
use ivynet_notifications::{NotificationConfig, NotificationDispatcher, OrganizationDatabase};
use sqlx::PgPool;

use crate::HeartbeatMonitor;

#[ivynet_grpc::async_trait]
impl<D: OrganizationDatabase> Heartbeat for HeartbeatMonitor<D> {
    async fn send_client_heartbeat(
        &self,
        request: Request<ClientHeartbeatSrc>,
    ) -> Result<Response<()>, Status> {
        let client_id = request.into_inner().try_into()?;
        self.post_client_heartbeat(client_id).await?;
        Ok(Response::new(()))
    }

    async fn send_machine_heartbeat(
        &self,
        request: Request<MachineHeartbeatSrc>,
    ) -> Result<Response<()>, Status> {
        let machine_id = request.into_inner().try_into()?;
        self.post_machine_heartbeat(machine_id).await?;
        Ok(Response::new(()))
    }

    async fn send_node_heartbeat(
        &self,
        request: Request<NodeHeartbeatSrc>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let node_id = req.try_into()?;
        self.post_node_heartbeat(node_id).await?;
        Ok(Response::new(()))
    }
}

#[allow(dead_code)]
async fn serve(
    tls_cert: Option<String>,
    tls_key: Option<String>,
    port: u16,
    db: PgPool,
    notification_config: NotificationConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let alert_db = AlertDb::new(db.clone());
    let notifier = NotificationDispatcher::new(notification_config, alert_db);
    let heartbeat_monitor = HeartbeatMonitor::new(db, Arc::new(notifier));
    let server = Server::new(HeartbeatServer::new(heartbeat_monitor), tls_cert, tls_key);
    let endpoint = Endpoint::Port(port);
    server.serve(endpoint).await?;
    Ok(())
}
