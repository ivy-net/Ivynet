-- Migration: Alter node_data table

-- Step 1: Drop the existing primary key
ALTER TABLE node_data DROP CONSTRAINT node_data_pkey;

-- Step 2: Drop the id column
ALTER TABLE node_data DROP COLUMN id;

-- Step 3: Add operator_id as the new primary key
ALTER TABLE node_data ADD COLUMN operator_id BYTEA PRIMARY KEY;

-- Step 6: Add unique constraint on (operator_id, avs_name)
ALTER TABLE node_data ADD CONSTRAINT unique_operator_avs UNIQUE (operator_id, avs_name);
