CREATE TYPE user_role AS ENUM ('owner', 'admin', 'user', 'reader');

CREATE TABLE IF NOT EXISTS account (
    user_id         BIGSERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organization
                        ON DELETE CASCADE,    
    email           TEXT UNIQUE NOT NULL,
    password        TEXT NOT NULL,
    role            user_role NOT NULL,
    created_at      TIMESTAMP NOT NULL,
    updated_at      TIMESTAMP NOT NULL
);
