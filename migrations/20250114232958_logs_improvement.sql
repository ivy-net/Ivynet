CREATE INDEX idx_log_created_at ON log (created_at DESC);
CREATE INDEX idx_log_composite ON log (machine_id, avs_name, created_at DESC);

ALTER TABLE log ALTER COLUMN log SET STORAGE EXTENDED;
