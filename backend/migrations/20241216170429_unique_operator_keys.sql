ALTER TABLE operator_keys
ADD CONSTRAINT uq_operator_keys_org_pubkey
UNIQUE (organization_id, public_key);