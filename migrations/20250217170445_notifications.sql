CREATE TABLE IF NOT EXISTS organization_notifications (
    organization_id BIGINT NOT NULL REFERENCES organization
                        ON DELETE CASCADE UNIQUE,
    email           BOOL NOT NULL DEFAULT FALSE,
    telegram        BOOL NOT NULL DEFAULT FALSE,
    pagerduty       BOOL NOT NULL DEFAULT FALSE,
    alert_flags     BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMP NOT NULL
);

CREATE INDEX idx_org_not ON organization_notifications (organization_id);

CREATE TYPE notification_type AS ENUM ('email', 'telegram', 'pagerduty');

CREATE TABLE IF NOT EXISTS notification_settings (
    id              SERIAL PRIMARY KEY,
    organization_id BIGINT NOT NULL REFERENCES organization
                        ON DELETE CASCADE,
    settings_type   notification_type NOT NULL,
    settings_value  TEXT NOT NULL, -- INTEGRATION KEY for pager duty a single email for email or telegram chat id for a chat
    created_at      TIMESTAMP NOT NULL
);

CREATE INDEX idx_notification_settings_org ON notification_settings (organization_id);
CREATE INDEX idx_notification_settings_or_type ON notification_settings (organization_id, settings_type);

