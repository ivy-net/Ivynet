CREATE TABLE operator_keys (
    id BIGSERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    public_key VARCHAR(512) NOT NULL
);

CREATE INDEX idx_org_keys_org_id ON operator_keys(organization_id);