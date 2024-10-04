CREATE TABLE IF NOT EXISTS node_data (
    id SERIAL PRIMARY KEY,
    node_id BYTEA NOT NULL,
    avs_name VARCHAR(255) NOT NULL,
    avs_version VARCHAR(50) NOT NULL,
    active_set BOOLEAN NOT NULL
);
