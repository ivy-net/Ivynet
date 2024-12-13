CREATE TABLE IF NOT EXISTS avs_active_set (
    directory BYTEA NOT NULL,
    operator  BYTEA NOT NULL,
    chain_id  BIGINT NOT NULL,
    active    BOOL NOT NULL,
    block     BIGINT NOT NULL,
    log_index BIGINT NOT NULL,
    PRIMARY KEY ( directory, operator, chain_id )
);

ALTER TABLE avs_active_set RENAME COLUMN directory TO avs;
ALTER TABLE avs_active_set ADD COLUMN directory BYTEA NOT NULL DEFAULT decode('00', 'hex');

CREATE INDEX avs_active_set_collection ON avs_active_set USING hash (directory);
