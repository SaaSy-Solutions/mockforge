-- Feature usage tracking

CREATE TYPE feature_type AS ENUM (
    'hosted_mock_deploy',
    'hosted_mock_request',
    'plugin_publish',
    'plugin_install',
    'template_publish',
    'template_install',
    'scenario_publish',
    'scenario_install',
    'api_token_create',
    'api_token_use',
    'billing_checkout',
    'billing_upgrade',
    'billing_downgrade',
    'org_create',
    'org_invite',
    'marketplace_search',
    'marketplace_download'
);

CREATE TABLE feature_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    feature feature_type NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_feature_usage_org ON feature_usage(org_id, created_at DESC);
CREATE INDEX idx_feature_usage_feature ON feature_usage(feature, created_at DESC);
CREATE INDEX idx_feature_usage_user ON feature_usage(user_id, created_at DESC);
CREATE INDEX idx_feature_usage_created ON feature_usage(created_at);

-- Cleanup old events (older than 90 days) - can be run periodically
-- DELETE FROM feature_usage WHERE created_at < NOW() - INTERVAL '90 days';
