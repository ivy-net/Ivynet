CREATE TABLE IF NOT EXISTS avs (
    avs_name          VARCHAR(250) NOT NULL,
    machine_id        UUID NOT NULL REFERENCES machine
                               ON DELETE CASCADE,
    avs_version       VARCHAR(50) NOT NULL,
    operator_address  BYTEA,
    active_set        BOOLEAN NOT NULL,
    created_at        TIMESTAMP NOT NULL,
    updated_at        TIMESTAMP NOT NULL,
    PRIMARY KEY (machine_id, avs_name)
);
