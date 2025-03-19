CREATE TABLE IF NOT EXISTS notification_settings (
    organization_id BIGINT NOT NULL REFERENCES organization
                        ON DELETE CASCADE UNIQUE,
    email           BOOL NOT NULL DEFAULT FALSE,
    telegram        BOOL NOT NULL DEFAULT FALSE,
    pagerduty       BOOL NOT NULL DEFAULT FALSE,
    alert_flags     BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMP NOT NULL
);

CREATE INDEX idx_notification_settings ON notification_settings (organization_id);

CREATE TYPE service_type AS ENUM ('email', 'telegram', 'pagerduty');

CREATE TABLE IF NOT EXISTS service_settings (
    id              UUID PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organization
                        ON DELETE CASCADE,
    settings_type   service_type NOT NULL,
    settings_value  TEXT NOT NULL, -- INTEGRATION KEY for pager duty a single email for email or telegram chat id for a chat
    created_at      TIMESTAMP NOT NULL
);

CREATE INDEX idx_service_settings_org ON service_settings (organization_id);
CREATE INDEX idx_service_settings_or_type ON service_settings (organization_id, settings_type);

-- Create a trigger function to create default notification settings when a new organization is created
CREATE OR REPLACE FUNCTION create_default_notification_settings()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notification_settings (
        organization_id,
        email,
        telegram,
        pagerduty,
        alert_flags,
        created_at,
        updated_at
    )
    VALUES (
        NEW.organization_id,
        FALSE,   -- default value for email
        FALSE,   -- default value for telegram
        FALSE,   -- default value for pagerduty
        0,       -- Alert flags off
        NOW(),   -- created_at timestamp
        NOW()    -- updated_at timestamp
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create the trigger on the organization table
CREATE TRIGGER after_organization_insert
AFTER INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION create_default_notification_settings();

-- Create a new notification_settings table for each existing organization
INSERT INTO notification_settings (
    organization_id,
    email,
    telegram,
    pagerduty,
    alert_flags,
    created_at,
    updated_at
)
SELECT
    o.organization_id,
    FALSE,      -- default value for email
    FALSE,      -- default value for telegram
    FALSE,      -- default value for pagerduty
    0,          -- default alert_flags
    NOW(),      -- current timestamp for created_at
    NOW()       -- current timestamp for updated_at
FROM
    organization o
WHERE
    NOT EXISTS (
        SELECT 1
        FROM notification_settings n
        WHERE n.organization_id = o.organization_id
    );
