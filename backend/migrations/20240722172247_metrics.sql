CREATE TABLE IF NOT EXISTS metric (
    node_id      BYTEA NOT NULL REFERENCES node
                        ON DELETE CASCADE,
    name         TEXT NOT NULL,
    value        DOUBLE PRECISION NOT NULL,
    attributes   JSONB,
    created_at   TIMESTAMP NOT NULL
);
