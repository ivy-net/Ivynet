CREATE TABLE IF NOT EXISTS node (
    node_id            BYTEA     PRIMARY KEY,
    organization_id    BIGINT    NOT NULL REFERENCES organization 
                                    ON DELETE CASCADE,
    created_at         TIMESTAMP NOT NULL,
    updated_at         TIMESTAMP NOT NULL
);
