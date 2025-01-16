-- Table for a particular node on a given machine.

CREATE TABLE IF NOT EXISTS avs (
    avs_name          VARCHAR(250) NOT NULL,
    avs_type          VARCHAR(50) NOT NULL DEFAULT 'unknown',
    machine_id        UUID NOT NULL REFERENCES machine
                               ON DELETE CASCADE,
    avs_version       VARCHAR(50) NOT NULL,
    version_hash      VARCHAR(250) NOT NULL,
    operator_address  BYTEA,
    chain             VARCHAR(50),
    active_set        BOOLEAN NOT NULL,
    created_at        TIMESTAMP NOT NULL,
    updated_at        TIMESTAMP NOT NULL,
    PRIMARY KEY (machine_id, avs_name)
);
