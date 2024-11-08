CREATE TABLE IF NOT EXISTS log (
    machine_id      UUID NOT NULL,
    avs_name        VARCHAR(250) NOT NULL,
    log             TEXT NOT NULL,
    log_level       log_level NOT NULL,
    created_at      TIMESTAMP NOT NULL,
    other_fields    JSONB,
    FOREIGN KEY (machine_id, avs_name) REFERENCES avs ON DELETE CASCADE
);
