-- Copy data in batches to avoid lock contention
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN SELECT DISTINCT machine_id FROM log
    LOOP
        -- Create partition for this machine
        PERFORM create_log_partition(r.machine_id);

        -- Copy data for this machine
        EXECUTE format(
            'INSERT INTO log_partitioned
             SELECT * FROM log
             WHERE machine_id = %L',
            r.machine_id
        );
    END LOOP;
END $$;
