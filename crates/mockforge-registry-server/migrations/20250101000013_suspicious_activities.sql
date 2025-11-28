-- Suspicious activity detection

CREATE TYPE suspicious_activity_type AS ENUM (
    'multiple_failed_logins',
    'login_from_new_location',
    'rapid_api_token_creation',
    'unusual_api_usage',
    'rapid_settings_changes',
    'unusual_billing_activity',
    'multiple_ip_addresses',
    'account_takeover_attempt',
    'brute_force_attempt',
    'unusual_deployment_pattern'
);

CREATE TABLE suspicious_activities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    activity_type suspicious_activity_type NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    description TEXT NOT NULL,
    metadata JSONB,
    ip_address VARCHAR(45), -- IPv6 max length
    user_agent TEXT,
    resolved BOOLEAN DEFAULT FALSE,
    resolved_at TIMESTAMP,
    resolved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_suspicious_activities_org ON suspicious_activities(org_id, created_at DESC);
CREATE INDEX idx_suspicious_activities_user ON suspicious_activities(user_id, created_at DESC);
CREATE INDEX idx_suspicious_activities_type ON suspicious_activities(activity_type, created_at DESC);
CREATE INDEX idx_suspicious_activities_severity ON suspicious_activities(severity, created_at DESC);
CREATE INDEX idx_suspicious_activities_resolved ON suspicious_activities(resolved, created_at DESC);
CREATE INDEX idx_suspicious_activities_created ON suspicious_activities(created_at);

-- Cleanup old resolved activities (older than 30 days) - can be run periodically
-- DELETE FROM suspicious_activities WHERE resolved = TRUE AND resolved_at < NOW() - INTERVAL '30 days';
