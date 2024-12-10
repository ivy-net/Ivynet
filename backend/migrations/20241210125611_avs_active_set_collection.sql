ALTER TABLE avs_active_set RENAME COLUMN directory TO avs;
ALTER TABLE avs_active_set ADD COLUMN directory BYTEA NOT NULL DEFAULT decode('00', 'hex');

CREATE INDEX avs_active_set_collection ON avs_active_set USING hash (directory);
