ALTER TABLE avs_active_set RENAME COLUMN directory TO avs;
ALTER TABLE avs_active_set ADD COLUMN directory BYTEA;

CREATE INDEX avs_active_set_collection ON avs_active_set USING hash (directory);
