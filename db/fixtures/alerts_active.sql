DO $$
DECLARE
    org_name text := 'MontyPython';
    machine_id uuid := 'dcbf22c7-9d96-47ac-bf06-62d6544e440d';
    client_id bytea := decode('0101010101010101010101010101010101010101', 'hex');
BEGIN
    INSERT INTO alerts_active (
        alert_type,
        machine_id,
        organization_id,
        client_id,
        node_name,
        created_at,
        custom_data
    ) VALUES (
        1,
        machine_id,
        (SELECT organization_id FROM organization WHERE name = org_name),
        client_id,
        'test_node',
        NOW(),
        '{"foo": "bar"}'
    );
END;
$$;
