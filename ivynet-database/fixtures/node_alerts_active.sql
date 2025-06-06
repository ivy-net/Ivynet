DO $$
DECLARE
    org_name text := 'MontyPython';
    machine_id uuid := 'dcbf22c7-9d96-47ac-bf06-62d6544e440d';
    client_id bytea := decode('0101010101010101010101010101010101010101', 'hex');
BEGIN
    INSERT INTO node_alerts_active (
        alert_id,
        machine_id,
        organization_id,
        client_id,
        node_name,
        created_at,
        alert_data,
        telegram_send,
        sendgrid_send,
        pagerduty_send
    ) VALUES (
        '00000000-0000-0000-0000-000000000001'::uuid,
        machine_id,
        (SELECT organization_id FROM organization WHERE name = org_name),
        client_id,
        'test_node',
        NOW(),
        '{"UnregisteredFromActiveSet": {"node_name": "test", "node_type": "test", "operator": "0x0000000000000000000000000000000000000000"}}',
        'no_send',
        'no_send',
        'no_send'
    );

     INSERT INTO node_alerts_active (
         alert_id,
         machine_id,
         organization_id,
         client_id,
         node_name,
         created_at,
         alert_data,
         telegram_send,
         sendgrid_send,
         pagerduty_send
     ) VALUES (
         '00000000-0000-0000-0000-000000000002'::uuid,
         machine_id,
         (SELECT organization_id FROM organization WHERE name = org_name),
         client_id,
         'test_node',
         NOW(),
        '{"UnregisteredFromActiveSet": {"node_name": "test", "node_type": "test", "operator": "0x0000000000000000000000000000000000000000"}}',
         'no_send',
         'no_send',
         'no_send'
     );

     INSERT INTO node_alerts_active (
         alert_id,
         machine_id,
         organization_id,
         client_id,
         node_name,
         created_at,
         alert_data,
         telegram_send,
         sendgrid_send,
         pagerduty_send
     ) VALUES (
         '00000000-0000-0000-0000-000000000003'::uuid,
         machine_id,
         (SELECT organization_id FROM organization WHERE name = org_name),
         client_id,
         'test_node',
         NOW(),
         '{"IdleMachine": {"machine_id": "dcbf22c7-9d96-47ac-bf06-62d6544e440d"}}',
         'no_send',
         'no_send',
         'no_send'
     );
END;
$$;
