ALTER TABLE sso_configurations
    ADD COLUMN IF NOT EXISTS domain_verified BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS domain_verification_token VARCHAR(255);
