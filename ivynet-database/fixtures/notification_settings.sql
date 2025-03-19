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
