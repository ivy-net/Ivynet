CREATE TABLE IF NOT EXISTS organization_alerts_historical (
    alert_db_id       BIGSERIAL NOT NULL,
    alert_id          UUID      NOT NULL,
    organization_id   BIGINT    NOT NULL REFERENCES organization
                                    ON DELETE CASCADE,
    created_at        TIMESTAMP NOT NULL,
    acknowledged_at   TIMESTAMP NOT NULL,
    alert_data        JSONB     NOT NULL,
    PRIMARY KEY (organization_id, alert_db_id)
) PARTITION BY LIST (organization_id);

-- Create the partition tables for existing organizations
DO $$
DECLARE
    org_id bigint;
BEGIN
    FOR org_id IN SELECT organization_id FROM organization LOOP
        EXECUTE format('CREATE TABLE IF NOT EXISTS organization_alerts_historical_%s PARTITION OF organization_alerts_historical FOR VALUES IN (%s);', org_id, org_id);
    END LOOP;
END $$;

-- Create the partition tables for new organizations
CREATE OR REPLACE FUNCTION create_organization_alerts_historical_partition()
RETURNS trigger AS $$
DECLARE
    partition_name text;
BEGIN
    partition_name := format('organization_alerts_historical_%s', NEW.organization_id);

    EXECUTE format(
       'CREATE TABLE IF NOT EXISTS %I PARTITION OF organization_alerts_historical FOR VALUES IN (%L);',
       partition_name,
       NEW.organization_id
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create and attach trigger to org
CREATE TRIGGER after_insert_organization_org_historical
AFTER INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION create_organization_alerts_historical_partition();