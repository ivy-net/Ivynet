CREATE TABLE IF NOT EXISTS alerts_historical (
    alert_id          BIGSERIAL,
    alert_type        INT       NOT NULL,
    machine_id        UUID      NOT NULL REFERENCES machine
                                    ON DELETE CASCADE,
    organization_id   BIGINT    NOT NULL REFERENCES organization
                                    ON DELETE CASCADE,
    client_id         BYTEA     NOT NULL REFERENCES client 
                                    ON DELETE CASCADE,
    node_name         VARCHAR(250) NOT NULL,
    created_at        TIMESTAMP NOT NULL,
    acknowledged_at   TIMESTAMP,
    resolved_at       TIMESTAMP NOT NULL,
    PRIMARY KEY (organization_id, alert_id)
) PARTITION BY LIST (organization_id);
