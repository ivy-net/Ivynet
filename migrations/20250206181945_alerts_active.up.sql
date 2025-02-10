CREATE TABLE IF NOT EXISTS alerts_active (
    alert_id            BIGSERIAL,
    alert_type          INT         NOT NULL,
    machine_id          UUID        NOT NULL REFERENCES machine
                                        ON DELETE CASCADE,
    organization_id     BIGINT      NOT NULL REFERENCES organization
                                        ON DELETE CASCADE,
    client_id           BYTEA       NOT NULL REFERENCES client
                                        ON DELETE CASCADE,
    node_name           VARCHAR(250) NOT NULL,
    created_at          TIMESTAMP   NOT NULL,
    acknowledged_at     TIMESTAMP,
    custom_data         JSONB,
    PRIMARY KEY (organization_id, alert_id)
) PARTITION BY LIST (organization_id);

-- Create the partition tables for existing organizations
DO $$
DECLARE
    org_id bigint;
BEGIN
    FOR org_id IN SELECT organization_id FROM organization LOOP
        EXECUTE format('CREATE TABLE IF NOT EXISTS alerts_active_%s PARTITION OF alerts_active FOR VALUES IN (%s);', org_id, org_id);
    END LOOP;
END $$;

-- Create the partition tables for new organizations
CREATE OR REPLACE FUNCTION create_alerts_active_partition()
RETURNS trigger AS $$
DECLARE
    partition_name text;
BEGIN
    partition_name := format('alerts_active_%s', NEW.organization_id);

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF alerts_active FOR VALUES IN (%L);',
        partition_name,
        NEW.organization_id
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create and attach trigger to org
CREATE TRIGGER after_insert_organization_active
AFTER INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION create_alerts_active_partition();
