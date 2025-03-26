-- Drop client heartbeat historical alerts
DROP TABLE IF EXISTS client_heartbeat_alerts_historical;
DROP TRIGGER IF EXISTS after_insert_organization_client_heartbeat_historical ON organization;
DROP FUNCTION IF EXISTS create_client_heartbeat_alerts_historical_partition();

-- Drop machine heartbeat historical alerts
DROP TABLE IF EXISTS machine_heartbeat_alerts_historical;
DROP TRIGGER IF EXISTS after_insert_organization_machine_heartbeat_historical ON organization;
DROP FUNCTION IF EXISTS create_machine_heartbeat_alerts_historical_partition();

-- Drop node heartbeat historical alerts
DROP TABLE IF EXISTS node_heartbeat_alerts_historical;
DROP TRIGGER IF EXISTS after_insert_organization_node_heartbeat_historical ON organization;
DROP FUNCTION IF EXISTS create_node_heartbeat_alerts_historical_partition();