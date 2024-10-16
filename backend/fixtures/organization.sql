CREATE TABLE IF NOT EXISTS organization (
    organization_id BIGSERIAL PRIMARY KEY,
    name            TEXT NOT NULL,
    verified        BOOL NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

INSERT INTO organization (name, verified, created_at, updated_at)
VALUES ('TestOrg', true, NOW(), NOW());
