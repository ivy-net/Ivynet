DO $$
DECLARE
    org_name text := 'MontyPython';
BEGIN
    -- Insert some test alerts into organization_alerts_active
    INSERT INTO organization_alerts_active (
        alert_id,
        organization_id,
        created_at,
        alert_data,
        telegram_send,
        sendgrid_send,
        pagerduty_send
    ) VALUES (
        '00000000-0000-0000-0000-000000000001'::uuid,
        (SELECT organization_id FROM organization WHERE name = org_name),
        NOW(),
        '{"Custom": {"node_name": "test_node_123123", "node_type": "test_type", "extra_data": "runtime_alert_fixture_1"}}',
        'no_send',
        'no_send',
        'no_send'
    );

    INSERT INTO organization_alerts_active (
        alert_id,
        organization_id,
        created_at,
        alert_data,
        telegram_send,
        sendgrid_send,
        pagerduty_send
    ) VALUES (
        '00000000-0000-0000-0000-000000000002'::uuid,
        (SELECT organization_id FROM organization WHERE name = org_name),
        NOW(),
        '{"Custom": {"node_name": "test_node_123123", "node_type": "test_type", "extra_data": "runtime_alert_fixture_2"}}',
        'no_send',
        'no_send',
        'no_send'
    );
END;
$$;
