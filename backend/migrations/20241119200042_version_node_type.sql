
-- Rename avs_name column in avs_version_data table
ALTER TABLE avs_version_data RENAME COLUMN avs_name TO node_type;

-- Update the unique constraint
ALTER TABLE avs_version_data
    DROP CONSTRAINT avs_version_data_avs_name_chain_key,
    ADD CONSTRAINT avs_version_data_node_type_chain_key UNIQUE (node_type, chain);