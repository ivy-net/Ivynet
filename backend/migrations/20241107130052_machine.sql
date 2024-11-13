CREATE TABLE IF NOT EXISTS machine (
    machine_id UUID  PRIMARY KEY,
    name       TEXT  NOT NULL,
    client_id  BYTEA NOT NULL REFERENCES client
                         ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
