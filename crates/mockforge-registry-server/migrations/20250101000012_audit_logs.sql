-- Audit logs for organization admin actions

CREATE TYPE audit_event_type AS ENUM (
    'member_added',
    'member_removed',
    'member_role_changed',
    'org_created',
    'org_updated',
    'org_deleted',
    'org_plan_changed',
    'billing_checkout',
    'billing_upgrade',
    'billing_downgrade',
    'billing_canceled',
    'api_token_created',
    'api_token_deleted',
    'api_token_rotated',
    'settings_updated',
    'byok_config_updated',
    'byok_config_deleted',
    'deployment_created',
    'deployment_deleted',
    'deployment_updated',
    'plugin_published',
    'template_published',
    'scenario_published',
    'password_changed',
    'email_changed',
    'two_factor_enabled',
    'two_factor_disabled',
    'admin_impersonation'
);

CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    event_type audit_event_type NOT NULL,
    description TEXT NOT NULL,
    metadata JSONB,
    ip_address VARCHAR(45), -- IPv6 max length
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_audit_logs_org ON audit_logs(org_id, created_at DESC);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id, created_at DESC);
CREATE INDEX idx_audit_logs_event_type ON audit_logs(event_type, created_at DESC);
CREATE INDEX idx_audit_logs_created ON audit_logs(created_at);

-- Cleanup old logs (older than 1 year) - can be run periodically
-- DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '1 year';
