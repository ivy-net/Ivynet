-- BEGIN;

-- -- Create new partitioned table with a different name
-- CREATE TABLE log_partitioned (
--     machine_id      UUID NOT NULL,
--     avs_name        VARCHAR(250) NOT NULL,
--     log             TEXT NOT NULL,
--     log_level       LOG_LEVEL NOT NULL,
--     created_at      TIMESTAMP NOT NULL,
--     other_fields    JSONB,
--     FOREIGN KEY (machine_id, avs_name) REFERENCES avs(machine_id, avs_name)
--         ON DELETE CASCADE
--         ON UPDATE CASCADE
-- ) PARTITION BY LIST (machine_id);

-- -- Add indexes to new table
-- CREATE INDEX idx_log_partitioned_created_at ON log_partitioned (created_at DESC);
-- CREATE INDEX idx_log_partitioned_composite ON log_partitioned (machine_id, avs_name, created_at DESC);

-- -- Create partition management function
-- CREATE OR REPLACE FUNCTION create_log_partition(
--     partition_machine_id UUID
-- ) RETURNS void AS $$
-- DECLARE
--     partition_name text;
-- BEGIN
--     partition_name := 'log_p_' || replace(partition_machine_id::text, '-', '_');
    
--     EXECUTE format(
--         'CREATE TABLE IF NOT EXISTS %I PARTITION OF log_partitioned FOR VALUES IN (%L)',
--         partition_name,
--         partition_machine_id
--     );
-- END;
-- $$ LANGUAGE plpgsql;

-- COMMIT;


-- Begin transaction block to ensure atomic migration
-- All operations will be rolled back if any step fails
BEGIN;

-- Create new partitioned logs table
-- This table stores application/service logs with machine-level partitioning
-- The partitioning strategy allows for:
--   1. Efficient machine-specific log queries
--   2. Independent retention policies per machine
--   3. Improved vacuum/maintenance operations
CREATE TABLE log_partitioned (
    machine_id      UUID NOT NULL,        -- Unique identifier for each machine, used as partition key
    avs_name        VARCHAR(250) NOT NULL, -- Application/Virtual Service identifier
    log             TEXT NOT NULL,        -- Actual log message content
    log_level       LOG_LEVEL NOT NULL,   -- Severity level (custom enum type)
    created_at      TIMESTAMP NOT NULL,   -- Log creation timestamp
    other_fields    JSONB,               -- Flexible schema for additional metadata
    -- Ensures referential integrity with parent 'avs' table
    -- CASCADE operations maintain data consistency automatically
    FOREIGN KEY (machine_id, avs_name) REFERENCES avs(machine_id, avs_name)
        ON DELETE CASCADE
        ON UPDATE CASCADE
) PARTITION BY LIST (machine_id);  -- Each machine gets its own partition

-- Index for time-based queries (e.g., "get most recent logs")
-- DESC ordering optimizes for recent log retrieval
CREATE INDEX idx_log_partitioned_created_at ON log_partitioned (created_at DESC);

-- Composite index supporting:
-- 1. Foreign key lookups
-- 2. Machine + service filtered queries
-- 3. Time-based filtering within machine+service context
CREATE INDEX idx_log_partitioned_composite ON log_partitioned (machine_id, avs_name, created_at DESC);

-- Function to dynamically create new partitions for machines
-- Parameters:
--   partition_machine_id: UUID of the machine needing a partition
-- Notes:
--   - Creates partition if it doesn't exist
--   - Partition names are sanitized UUIDs (hyphens replaced with underscores)
--   - Uses dynamic SQL for partition creation
-- Usage:
--   SELECT create_log_partition('123e4567-e89b-12d3-a456-426614174000');
CREATE OR REPLACE FUNCTION create_log_partition(
    partition_machine_id UUID
) RETURNS void AS $$
DECLARE
    partition_name text;
BEGIN
    -- Convert UUID to valid PostgreSQL identifier
    partition_name := 'log_p_' || replace(partition_machine_id::text, '-', '_');
    
    -- Create partition using dynamic SQL
    -- Note: format() handles proper escaping of identifiers and literals
    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF log_partitioned FOR VALUES IN (%L)',
        partition_name,
        partition_machine_id
    );
END;
$$ LANGUAGE plpgsql;

-- Commit all changes
-- Note: This migration requires exclusive locks on the affected tables
COMMIT;