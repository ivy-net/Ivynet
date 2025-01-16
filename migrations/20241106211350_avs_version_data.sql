CREATE TABLE avs_version_data (
    id                          SERIAL PRIMARY KEY,
    node_type                   VARCHAR(255) NOT NULL,
    chain                       VARCHAR(255) NOT NULL,
    latest_version_tag          VARCHAR(255) NOT NULL,
    latest_version_digest       VARCHAR(100) NOT NULL,
    breaking_change_tag         VARCHAR(255),
    breaking_change_datetime    TIMESTAMP
);

ALTER TABLE avs_version_data
    ADD CONSTRAINT avs_version_data_node_type_chain UNIQUE (node_type, chain);
