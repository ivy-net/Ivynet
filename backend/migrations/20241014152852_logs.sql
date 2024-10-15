CREATE TYPE log_level AS ENUM ('debug', 'info', 'warning', 'error', 'unknown');

CREATE TABLE IF NOT EXISTS log (
    node_id         BYTEA NOT NULL REFERENCES node
                        ON DELETE CASCADE,
    container_id    TEXT,
    container_name  TEXT NOT NULL,
    log             TEXT NOT NULL,
    log_level       log_level NOT NULL,
    created_at      TIMESTAMP NOT NULL,
    other_fields    JSONB
);
