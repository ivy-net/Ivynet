CREATE TABLE IF NOT EXISTS machine_alerts_active (
    alert_id            UUID         NOT NULL,
    machine_id          UUID         NOT NULL REFERENCES machine
                                        ON DELETE CASCADE,
    organization_id     BIGINT       NOT NULL REFERENCES organization
                                        ON DELETE CASCADE,
    client_id           BYTEA        NOT NULL REFERENCES client
                                        ON DELETE CASCADE,
    created_at          TIMESTAMP    NOT NULL,
    acknowledged_at     TIMESTAMP,
    alert_data          JSONB        NOT NULL,
    telegram_send       SEND_STATE   NOT NULL,
    sendgrid_send       SEND_STATE   NOT NULL,
    pagerduty_send      SEND_STATE   NOT NULL,

    PRIMARY KEY (organization_id, alert_id)
) PARTITION BY LIST (organization_id);

-- Create the partition tables for existing organizations
DO $$
DECLARE
    org_id bigint;
BEGIN
    FOR org_id IN SELECT organization_id FROM organization LOOP
        EXECUTE format('CREATE TABLE IF NOT EXISTS machine_alerts_active_%s PARTITION OF machine_alerts_active FOR VALUES IN (%s);', org_id, org_id);
    END LOOP;
END $$;

-- Create the partition tables for new organizations
CREATE OR REPLACE FUNCTION create_machine_alerts_active_partition()
RETURNS trigger AS $$
DECLARE
    partition_name text;
BEGIN
    partition_name := format('machine_alerts_active_%s', NEW.organization_id);

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF machine_alerts_active FOR VALUES IN (%L);',
        partition_name,
        NEW.organization_id
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create and attach trigger to org
CREATE TRIGGER after_insert_organization_machine_active
AFTER INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION create_machine_alerts_active_partition();