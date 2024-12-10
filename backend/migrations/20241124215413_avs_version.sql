CREATE TABLE IF NOT EXISTS avs_version_hash (
    id         BIGSERIAL PRIMARY KEY,
    hash       VARCHAR(250) NOT NULL,
    avs_type   VARCHAR(50) NOT NULL,
    version    VARCHAR(100) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

ALTER TABLE avs_version_hash
ADD CONSTRAINT unique_avs_type_version_hash UNIQUE (avs_type, version);
