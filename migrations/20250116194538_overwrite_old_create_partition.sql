-- 20250116xxxxx_fix_partition_function.sql
CREATE OR REPLACE FUNCTION create_log_partition(
    partition_machine_id UUID
) RETURNS void AS $$
DECLARE
    partition_name text;
BEGIN
    partition_name := 'log_p_' || replace(partition_machine_id::text, '-', '_');

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF log FOR VALUES IN (%L)',
        partition_name,
        partition_machine_id
    );
END;
$$ LANGUAGE plpgsql;