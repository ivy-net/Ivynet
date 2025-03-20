-- Add notification settings for our test organization
INSERT INTO notification_settings (
    organization_id,
    email,
    telegram,
    pagerduty,
    alert_flags,
    created_at,
    updated_at
) VALUES (
    1, -- Organization ID from new_user_registration.sql
    true,
    false,
    true,
    2,
    NOW(),
    NOW()
) ON CONFLICT (organization_id) DO UPDATE SET
    email = EXCLUDED.email,
    telegram = EXCLUDED.telegram,
    pagerduty = EXCLUDED.pagerduty,
    alert_flags = EXCLUDED.alert_flags,
    updated_at = NOW();

-- Add service settings with dummy UUIDs
INSERT INTO service_settings
    (id, organization_id, settings_type, settings_value, created_at)
VALUES
    ('af21425b-170c-56d1-b8b4-36cb1b7cacf0', 1, 'email', 'test1@example.com', NOW());

INSERT INTO service_settings
    (id, organization_id, settings_type, settings_value, created_at)
VALUES
    ('0928f7d5-c72c-5c91-8f7f-d1f1bd822733', 1, 'email', 'test2@example.com', NOW());

INSERT INTO service_settings
    (id, organization_id, settings_type, settings_value, created_at)
VALUES
    ('055e0a89-dfa2-582c-9caf-eeeb2aa730c1', 1, 'pagerduty', 'pdkey123', NOW());
