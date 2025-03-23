DROP TABLE IF EXISTS client_heartbeat_alerts;
DROP TRIGGER IF EXISTS after_insert_organization_client_heartbeat ON organization;
DROP FUNCTION IF EXISTS create_client_heartbeat_partition();

DROP TABLE IF EXISTS machine_heartbeat_alerts;
DROP TRIGGER IF EXISTS after_insert_organization_machine_heartbeat_alerts ON organization;
DROP FUNCTION IF EXISTS create_machine_heartbeat_alerts_partition();

DROP TABLE IF EXISTS node_heartbeat_alerts;
DROP TRIGGER IF EXISTS after_insert_organization_node_heartbeat_alerts ON organization;
DROP FUNCTION IF EXISTS create_node_heartbeat_alerts_partition();
