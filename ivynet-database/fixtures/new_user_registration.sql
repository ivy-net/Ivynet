-- setup org, user, and machine
DO $$
declare
    org_name text := 'MontyPython';
    machine_id uuid := 'dcbf22c7-9d96-47ac-bf06-62d6544e440d';
    client_id bytea := decode('0101010101010101010101010101010101010101', 'hex');

BEGIN

INSERT INTO organization (
    name,
    verified,
    created_at,
    updated_at
) VALUES (
    org_name,
    true,
    NOW(),
    NOW()
);

INSERT INTO client (
    client_id,
    organization_id,
    created_at,
    updated_at
) VALUES (
    client_id,
    (SELECT organization_id FROM organization WHERE name = org_name),
    NOW(),
    NOW()
);

INSERT INTO machine (
    machine_id,
    name,
    client_id,
    created_at,
    updated_at
) VALUES (
    machine_id,
    'test_machine',
    client_id,
    NOW(),
    NOW()
);

END$$;
