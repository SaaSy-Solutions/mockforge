-- Migration: SAML Security Enhancements
-- Adds tables for replay attack prevention and assertion tracking

-- Table to track used SAML assertion IDs to prevent replay attacks
CREATE TABLE IF NOT EXISTS saml_assertion_ids (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    assertion_id TEXT NOT NULL,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    name_id TEXT,
    issued_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(assertion_id, org_id)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_saml_assertion_ids_assertion_id ON saml_assertion_ids(assertion_id);
CREATE INDEX IF NOT EXISTS idx_saml_assertion_ids_org_id ON saml_assertion_ids(org_id);
CREATE INDEX IF NOT EXISTS idx_saml_assertion_ids_expires_at ON saml_assertion_ids(expires_at);
CREATE INDEX IF NOT EXISTS idx_saml_assertion_ids_assertion_id_org ON saml_assertion_ids(assertion_id, org_id);

-- Cleanup function to remove expired assertion IDs (older than 24 hours)
-- This should be run periodically via a cron job or scheduled task
CREATE OR REPLACE FUNCTION cleanup_expired_saml_assertions()
RETURNS void AS $$
BEGIN
    DELETE FROM saml_assertion_ids
    WHERE expires_at < NOW() - INTERVAL '24 hours';
END;
$$ LANGUAGE plpgsql;
