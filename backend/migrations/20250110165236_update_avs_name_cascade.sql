-- Add migration script here

BEGIN;

-- First drop the existing constraint
ALTER TABLE log 
DROP CONSTRAINT log_machine_id_avs_name_fkey;

-- Add the constraint back with ON UPDATE CASCADE
ALTER TABLE log 
ADD CONSTRAINT log_machine_id_avs_name_fkey 
FOREIGN KEY (machine_id, avs_name) 
REFERENCES avs(machine_id, avs_name) 
ON DELETE CASCADE 
ON UPDATE CASCADE;

COMMIT;