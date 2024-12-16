-- Delete duplicates keeping the one with lowest id (oldest)
DELETE FROM operator_keys
WHERE id IN (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                   PARTITION BY organization_id, public_key
                   ORDER BY id
               ) as row_num
        FROM operator_keys
    ) t
    WHERE t.row_num > 1
);

-- Add unique constraint
ALTER TABLE operator_keys
ADD CONSTRAINT uq_operator_keys_org_pubkey
UNIQUE (organization_id, public_key);