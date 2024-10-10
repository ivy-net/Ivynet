-- Migration: Alter node_data table

-- Step 1: Drop the existing primary key
ALTER TABLE node_data DROP CONSTRAINT node_data_pkey;

-- Step 2: Drop the id column
ALTER TABLE node_data DROP COLUMN id;

-- Step 3: Add operator_id as the new primary key
ALTER TABLE node_data ADD COLUMN operator_id BYTEA PRIMARY KEY;

-- Step 4: Copy data from node_id to operator_id
UPDATE node_data SET operator_id = node_id;

-- Step 5: Drop the node_id column
ALTER TABLE node_data DROP COLUMN node_id;

-- Step 6: Add unique constraint on (operator_id, avs_name)
ALTER TABLE node_data ADD CONSTRAINT unique_operator_avs UNIQUE (operator_id, avs_name);