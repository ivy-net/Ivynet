ALTER TABLE avs_active_set ADD COLUMN collection BYTEA;
CREATE INDEX avs_active_set_collection ON avs_active_set USING hash (collection);
