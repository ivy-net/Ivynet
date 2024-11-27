CREATE TABLE IF NOT EXISTS avs_version_hash (
    hash       VARCHAR(250) PRIMARY KEY,
    avs_type   VARCHAR(50) NOT NULL,
    version    VARCHAR(100) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
