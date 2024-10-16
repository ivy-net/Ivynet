CREATE TABLE IF NOT EXISTS node (
    node_id BYTEA PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organization
        ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

-- Insert a sample node, using the organization created by organization.sql
INSERT INTO node (node_id, organization_id, created_at, updated_at)
VALUES (
    decode('00000000000000000000000000000000deadbeef', 'hex'), -- 'test-node-id' in hex
    (SELECT organization_id FROM organization WHERE name = 'TestOrg'),
    NOW(), NOW()
);
