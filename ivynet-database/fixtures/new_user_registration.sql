-- setup org, user, and machine
DO $$
declare
    org_name text := 'MontyPython';
    machine_id uuid := 'dcbf22c7-9d96-47ac-bf06-62d6544e440d';
    client_id bytea := decode('0101010101010101010101010101010101010101', 'hex');

    org_name_2 text := 'TheNightsWhoSayNi';
    machine_id_2 uuid := 'd160619b-5fb8-4507-b73a-e2f5bd05d477';
    client_id_2 bytea := decode('0101010101010101010101010101010101010102', 'hex');

BEGIN

-- org 1
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

-- org 2
INSERT INTO organization (
    name,
    verified,
    created_at,
    updated_at
) VALUES (
    org_name_2,
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
    client_id_2,
    (SELECT organization_id FROM organization WHERE name = org_name_2),
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
    machine_id_2,
    'test_machine_2',
    client_id_2,
    NOW(),
    NOW()
);

END$$;
