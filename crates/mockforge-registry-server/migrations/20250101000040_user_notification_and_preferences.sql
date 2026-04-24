-- Per-user notification toggles and free-form UI preferences.
--
-- `email_notifications` gates optional transactional emails (welcome,
-- subscription updates, token-rotation reminders). Critical emails
-- (password reset, email verification, 2FA setup) are always sent.
--
-- `security_alerts` gates notifications about account-level security
-- events (e.g., password changed, 2FA enabled/disabled).
--
-- `preferences` stores the admin UI preferences blob. Shape is owned by
-- the UI; the server just round-trips and merges partial updates.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS email_notifications BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS security_alerts BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS preferences JSONB NOT NULL DEFAULT '{}'::jsonb;

COMMENT ON COLUMN users.email_notifications IS 'Opt-in for optional transactional emails';
COMMENT ON COLUMN users.security_alerts IS 'Opt-in for security-related account change notifications';
COMMENT ON COLUMN users.preferences IS 'Free-form admin UI preferences (JSON blob)';
