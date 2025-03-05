DO $$
DECLARE
    org_name text := 'MontyPython';
    machine_id uuid := 'dcbf22c7-9d96-47ac-bf06-62d6544e440d';
    client_id bytea := decode('0101010101010101010101010101010101010101', 'hex');
BEGIN
    INSERT INTO client_log (
        client_id,
        log,
        log_level,
        created_at,
        other_fields
    ) VALUES (
        client_id,
        'TEST_LOG_MSG',
        'info',
        NOW(),
        '{"foo": "bar"}'
    );
END;
$$;
