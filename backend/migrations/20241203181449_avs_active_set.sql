CREATE TABLE IF NOT EXISTS avs_active_set (
    avs       BYTEA NOT NULL,
    directory BYTEA NOT NULL DEFAULT decode('00', 'hex'),
    operator  BYTEA NOT NULL,
    chain_id  BIGINT NOT NULL,
    active    BOOL NOT NULL,
    block     BIGINT NOT NULL,
    log_index BIGINT NOT NULL,
    PRIMARY KEY ( directory, operator, chain_id )
);

CREATE INDEX avs_active_set_collection ON avs_active_set USING hash (directory);
