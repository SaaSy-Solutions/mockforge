-- Add an optional email_domain to SSO configurations so the pre-login
-- discovery endpoint (GET /api/v1/sso/discover?email=...) can map a user's
-- email domain to the org + provider to redirect them to. Domain OWNERSHIP is
-- still verified live via DNS (the #833/#746/#778 gate); this column is only the
-- routing key for discovery and is never trusted as proof of ownership.

ALTER TABLE sso_configurations
    ADD COLUMN email_domain VARCHAR(255);

-- Normalize to lowercase for case-insensitive matching against the email domain.
-- Partial unique index: a verified domain can only route to one org, but many
-- rows may have NULL (no domain configured).
CREATE UNIQUE INDEX idx_sso_configs_email_domain
    ON sso_configurations (LOWER(email_domain))
    WHERE email_domain IS NOT NULL;
