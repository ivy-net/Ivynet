DROP TABLE IF EXISTS avs_version_data;

CREATE TABLE avs_version_data (
    id SERIAL PRIMARY KEY,
    avs_name VARCHAR(255) NOT NULL,
    chain VARCHAR(255) NOT NULL,
    latest_version_tag VARCHAR(255) NOT NULL,
    latest_version_digest VARCHAR(100) NOT NULL,
    breaking_change_tag VARCHAR(255),
    breaking_change_datetime TIMESTAMP,
    UNIQUE(avs_name, chain)
);
