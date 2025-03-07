CREATE TABLE IF NOT EXISTS organization_alerts_active (
    alert_id            UUID         NOT NULL,
    organization_id     BIGINT       NOT NULL REFERENCES organization
                                        ON DELETE CASCADE,
    created_at          TIMESTAMP    NOT NULL,
    alert_data          JSONB        NOT NULL,
    PRIMARY KEY (organization_id, alert_id)
) PARTITION BY LIST (organization_id);

-- Create the partition tables for existing organizations
DO $$
DECLARE
    org_id bigint;
BEGIN
    FOR org_id IN SELECT organization_id FROM organization LOOP
        EXECUTE format('CREATE TABLE IF NOT EXISTS organization_alerts_active_%s PARTITION OF organization_alerts_active FOR VALUES IN (%s);', org_id, org_id);
    END LOOP;
END $$;

-- Create the partition tables for new organizations
CREATE OR REPLACE FUNCTION create_organization_alerts_active_partition()
RETURNS trigger AS $$
DECLARE
    partition_name text;
BEGIN
    partition_name := format('organization_alerts_active_%s', NEW.organization_id);

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF organization_alerts_active FOR VALUES IN (%L);',
        partition_name,
        NEW.organization_id
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create and attach trigger to org
CREATE TRIGGER after_insert_organization_org_active
AFTER INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION create_organization_alerts_active_partition();