-- Client heartbeats historical
CREATE TABLE IF NOT EXISTS client_heartbeat_alerts_historical (
    id                  BIGSERIAL    NOT NULL,
    client_id           BYTEA        NOT NULL,
    organization_id     BIGINT       NOT NULL REFERENCES organization
                                        ON DELETE CASCADE,
    last_response_time  TIMESTAMP    NOT NULL,
    created_at          TIMESTAMP    NOT NULL,
    resolved_at         TIMESTAMP    NOT NULL,
    PRIMARY KEY (id, organization_id)
) PARTITION BY LIST (organization_id);

-- Create the partition tables for existing organizations
DO $$
DECLARE
    org_id bigint;
BEGIN
    FOR org_id IN SELECT organization_id FROM organization LOOP
        EXECUTE format('CREATE TABLE IF NOT EXISTS client_heartbeat_alerts_historical_%s PARTITION OF client_heartbeat_alerts_historical FOR VALUES IN (%s);', org_id, org_id);
    END LOOP;
END $$;

-- Create the partition tables for new organizations
CREATE OR REPLACE FUNCTION create_client_heartbeat_historical_partition()
RETURNS trigger AS $$
DECLARE
    partition_name text;
BEGIN
    partition_name := format('client_heartbeat_alerts_historical_%s', NEW.organization_id);

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF client_heartbeat_alerts_historical FOR VALUES IN (%L);',
        partition_name,
        NEW.organization_id
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create and attach trigger to org
CREATE TRIGGER after_insert_organization_client_heartbeat_historical
AFTER INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION create_client_heartbeat_historical_partition();


-- Machine heartbeats historical
CREATE TABLE IF NOT EXISTS machine_heartbeat_alerts_historical (
    id                  BIGSERIAL    NOT NULL,
    machine_id          UUID         NOT NULL,
    organization_id     BIGINT       NOT NULL REFERENCES organization
                                        ON DELETE CASCADE,
    last_response_time  TIMESTAMP    NOT NULL,
    created_at          TIMESTAMP    NOT NULL,
    resolved_at         TIMESTAMP    NOT NULL,
    PRIMARY KEY (id, organization_id)
) PARTITION BY LIST (organization_id);

-- Create the partition tables for existing organizations
DO $$
DECLARE
    org_id bigint;
BEGIN
    FOR org_id IN SELECT organization_id FROM organization
    LOOP
        EXECUTE format('CREATE TABLE IF NOT EXISTS machine_heartbeat_alerts_historical_%s PARTITION OF machine_heartbeat_alerts_historical FOR VALUES IN (%s);', org_id, org_id);
    END LOOP;
END $$;

-- Create the partition tables for new organizations
CREATE OR REPLACE FUNCTION create_machine_heartbeat_alerts_historical_partition()
RETURNS trigger AS $$
DECLARE
    partition_name text;
BEGIN
    partition_name := format('machine_heartbeat_alerts_historical_%s', NEW.organization_id);

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF machine_heartbeat_alerts_historical FOR VALUES IN (%L);',
        partition_name,
        NEW.organization_id
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create and attach trigger to org
CREATE TRIGGER after_insert_organization_machine_heartbeat_alerts_historical
AFTER INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION create_machine_heartbeat_alerts_historical_partition();


-- Node heartbeats historical
CREATE TABLE IF NOT EXISTS node_heartbeat_alerts_historical (
    id                  BIGSERIAL    NOT NULL,
    node_id             TEXT         NOT NULL, -- UUID:NODE_NAME in plain string
    organization_id     BIGINT       NOT NULL REFERENCES organization
                                        ON DELETE CASCADE,
    last_response_time  TIMESTAMP    NOT NULL,
    created_at          TIMESTAMP    NOT NULL,
    resolved_at         TIMESTAMP    NOT NULL,
    PRIMARY KEY (id, organization_id)
) PARTITION BY LIST (organization_id);

-- Create the partition tables for existing organizations
DO $$
DECLARE
    org_id bigint;
BEGIN
    FOR org_id IN SELECT organization_id FROM organization
    LOOP
        EXECUTE format('CREATE TABLE IF NOT EXISTS node_heartbeat_alerts_historical_%s PARTITION OF node_heartbeat_alerts_historical FOR VALUES IN (%s);', org_id, org_id);
    END LOOP;
END $$;

-- Create the partition tables for new organizations
CREATE OR REPLACE FUNCTION create_node_heartbeat_alerts_historical_partition()
RETURNS trigger AS $$
DECLARE
    partition_name text;
BEGIN
    partition_name := format('node_heartbeat_alerts_historical_%s', NEW.organization_id);

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF node_heartbeat_alerts_historical FOR VALUES IN (%L);',
        partition_name,
        NEW.organization_id
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create and attach trigger to org
CREATE TRIGGER after_insert_organization_node_heartbeat_alerts_historical
AFTER INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION create_node_heartbeat_alerts_historical_partition();