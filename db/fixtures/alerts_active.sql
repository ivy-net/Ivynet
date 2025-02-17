DO $$
DECLARE
    org_name text := 'MontyPython';
    machine_id uuid := 'dcbf22c7-9d96-47ac-bf06-62d6544e440d';
    client_id bytea := decode('0101010101010101010101010101010101010101', 'hex');
BEGIN
    INSERT INTO alerts_active (
        alert_id,
        alert_type,
        machine_id,
        organization_id,
        client_id,
        node_name,
        created_at,
        custom_data
    ) VALUES (
        '00000000-0000-0000-0000-000000000001'::uuid,
        1,
        machine_id,
        (SELECT organization_id FROM organization WHERE name = org_name),
        client_id,
        'test_node',
        NOW(),
        '{"Custom": "test"}'
    );

    INSERT INTO alerts_active (
        alert_id,
        alert_type,
        machine_id,
        organization_id,
        client_id,
        node_name,
        created_at,
        custom_data
    ) VALUES (
        '00000000-0000-0000-0000-000000000002'::uuid,
        2,
        machine_id,
        (SELECT organization_id FROM organization WHERE name = org_name),
        client_id,
        'test_node',
        NOW(),
        '{"UnregisteredFromActiveSet": {"avs": "test", "address": "0x0000000000000000000000000000000000000000"}}'
    );

    INSERT INTO alerts_active (
        alert_id,
        alert_type,
        machine_id,
        organization_id,
        client_id,
        node_name,
        created_at,
        custom_data
    ) VALUES (
        '00000000-0000-0000-0000-000000000003'::uuid,
        3,
        machine_id,
        (SELECT organization_id FROM organization WHERE name = org_name),
        client_id,
        'test_node',
        NOW(),
        '"MachineNotResponding"'
    );
END;
$$;
