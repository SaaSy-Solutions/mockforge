-- Email-domain discovery for SSO: map a work-email domain to the org's IdP.
ALTER TABLE sso_configurations
    ADD COLUMN IF NOT EXISTS email_domain VARCHAR(255);

-- One domain maps to at most one org (deterministic discovery). Partial unique
-- index so it only applies to rows that set a domain.
CREATE UNIQUE INDEX IF NOT EXISTS idx_sso_email_domain
    ON sso_configurations (lower(email_domain))
    WHERE email_domain IS NOT NULL;
