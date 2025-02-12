CREATE TABLE IF NOT EXISTS client_log (
    client_id       BYTEA NOT NULL,
    log             TEXT NOT NULL,
    log_level       LOG_LEVEL NOT NULL,
    created_at      TIMESTAMP NOT NULL,
    other_fields    JSONB,
    FOREIGN KEY (client_id) REFERENCES client ON DELETE CASCADE
) PARTITION BY LIST (client_id);

CREATE INDEX idx_client_log_created_at ON client_log (created_at DESC);
CREATE INDEX idx_client_log_composite ON client_log (client_id, created_at DESC);

ALTER TABLE client_log ALTER COLUMN log SET STORAGE EXTENDED;

-- Create the partition tables for existing organizations
DO $$
DECLARE
    cid BYTEA;
BEGIN
    FOR cid IN SELECT client_id FROM client LOOP
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS client_log_%s PARTITION OF client_log FOR VALUES IN (%L);',
            encode(cid, 'hex'),  -- hex encoding for the partition name
            cid
        );
    END LOOP;
END $$;

-- Create the partition tables for new organizations
CREATE OR REPLACE FUNCTION create_client_logs_partition()
RETURNS trigger AS $$
DECLARE
    partition_name text;
BEGIN
    partition_name := format('client_log_%s', encode(NEW.client_id, 'hex'));

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF client_log FOR VALUES IN (%L);',
        partition_name,
        NEW.client_id
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create and attach trigger to org
CREATE TRIGGER after_insert_client_id
AFTER INSERT ON client 
FOR EACH ROW
EXECUTE FUNCTION create_client_logs_partition();
