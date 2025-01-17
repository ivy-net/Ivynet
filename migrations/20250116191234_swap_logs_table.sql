BEGIN;

-- Quick table swap
ALTER TABLE log RENAME TO log_old;
ALTER TABLE log_partitioned RENAME TO log;

COMMIT;
