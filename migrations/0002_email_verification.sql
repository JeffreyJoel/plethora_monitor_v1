-- Email verification for notification channels
-- Adds verification status and token fields to track email ownership

ALTER TABLE notification_channels ADD COLUMN IF NOT EXISTS verified BOOLEAN DEFAULT false;
ALTER TABLE notification_channels ADD COLUMN IF NOT EXISTS verification_token VARCHAR(64);
ALTER TABLE notification_channels ADD COLUMN IF NOT EXISTS verification_token_expires_at TIMESTAMP;

-- Email channels start unverified, other types are auto-verified
-- UPDATE notification_channels SET verified = true WHERE type != 'email';

-- Index for fast token lookups during verification
-- CREATE INDEX idx_notification_channels_verification_token ON notification_channels(verification_token) WHERE verification_token IS NOT NULL;

