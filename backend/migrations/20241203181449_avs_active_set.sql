CREATE TABLE IF NOT EXISTS avs_active_set (
    directory BYTEA NOT NULL,
    operator  BYTEA NOT NULL,
    chain_id  BIGINT NOT NULL,
    active    BOOL NOT NULL,
    block     BIGINT NOT NULL,
    log_index BIGINT NOT NULL,
    PRIMARY KEY ( directory, operator, chain_id )
);
