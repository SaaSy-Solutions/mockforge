-- Single Sign-On (SSO) support for Team plans
-- Adds SAML 2.0 SSO configuration per organization

-- SSO configurations table
CREATE TABLE sso_configurations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL DEFAULT 'saml' CHECK (provider IN ('saml', 'oidc')),
    enabled BOOLEAN DEFAULT FALSE,

    -- SAML 2.0 configuration
    saml_entity_id VARCHAR(255), -- SP Entity ID (our identifier)
    saml_sso_url TEXT, -- IdP SSO URL
    saml_slo_url TEXT, -- IdP SLO URL (optional)
    saml_x509_cert TEXT, -- IdP X.509 certificate (for signature verification)
    saml_name_id_format VARCHAR(255) DEFAULT 'urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress',

    -- OIDC configuration (for future use)
    oidc_issuer_url TEXT,
    oidc_client_id VARCHAR(255),
    oidc_client_secret VARCHAR(255),

    -- Attribute mapping
    attribute_mapping JSONB DEFAULT '{}'::jsonb, -- Maps IdP attributes to user fields

    -- Security settings
    require_signed_assertions BOOLEAN DEFAULT TRUE,
    require_signed_responses BOOLEAN DEFAULT TRUE,
    allow_unsolicited_responses BOOLEAN DEFAULT FALSE,

    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),

    UNIQUE(org_id) -- One SSO config per organization
);

-- SSO sessions table (for tracking active SSO sessions)
CREATE TABLE sso_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_index VARCHAR(255), -- SAML SessionIndex
    name_id VARCHAR(255), -- SAML NameID
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_sso_configs_org ON sso_configurations(org_id);
CREATE INDEX idx_sso_configs_enabled ON sso_configurations(org_id) WHERE enabled = TRUE;
CREATE INDEX idx_sso_sessions_org_user ON sso_sessions(org_id, user_id);
CREATE INDEX idx_sso_sessions_expires ON sso_sessions(expires_at);

-- Add comment for documentation
COMMENT ON TABLE sso_configurations IS 'SSO configuration for organizations (Team plan only)';
COMMENT ON TABLE sso_sessions IS 'Active SSO sessions for users authenticated via SSO';
