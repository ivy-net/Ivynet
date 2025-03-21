DROP TABLE client_heartbeat;
DROP TRIGGER after_insert_organization_client_heartbeat ON organization;

DROP TABLE machine_heartbeat;
DROP TRIGGER after_insert_organization_machine_heartbeat ON organization;

DROP TABLE node_heartbeat;
DROP TRIGGER after_insert_organization_node_heartbeat ON organization;
