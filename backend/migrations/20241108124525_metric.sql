CREATE TABLE IF NOT EXISTS metric (
    machine_id   UUID NOT NULL REFERENCES machine
                        ON DELETE CASCADE,
    avs_name     VARCHAR(250),
    name         TEXT NOT NULL,
    value        DOUBLE PRECISION NOT NULL,
    attributes   JSONB,
    created_at   TIMESTAMP NOT NULL
);
