-- Migration: Alter node_data table

-- Step 1: Add operator_id column
ALTER TABLE node_data ADD COLUMN operator_id BYTEA;

-- Step 2: Add unique constraint on (operator_id, avs_name)
ALTER TABLE node_data ADD CONSTRAINT unique_operator_avs UNIQUE (operator_id, avs_name);