CREATE TYPE verification_kind AS ENUM ('organization', 'user');

CREATE TABLE IF NOT EXISTS verification (
    verification_id   UUID PRIMARY KEY,
    associated_id     BIGINT NOT NULL,
    verification_type verification_kind NOT NULL,
    created_at        TIMESTAMP NOT NULL,
    updated_at        TIMESTAMP NOT NULL
);
