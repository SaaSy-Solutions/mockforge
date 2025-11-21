-- Organization AI Controls schema
-- Enables org-level management of AI usage, budgets, rate limits, and feature toggles
-- Supports YAML defaults with DB authoritative overrides (DB overrides YAML)

-- Organization AI budgets table
CREATE TABLE IF NOT EXISTS org_ai_budgets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE, -- NULL for org-level, specific UUID for workspace-level
    max_tokens_per_period BIGINT NOT NULL DEFAULT 1000000, -- Maximum tokens per period
    period_type VARCHAR(50) NOT NULL DEFAULT 'month', -- 'day', 'week', 'month', 'year'
    max_calls_per_period BIGINT NOT NULL DEFAULT 10000, -- Maximum AI calls per period
    current_tokens_used BIGINT DEFAULT 0, -- Current usage for this period
    current_calls_used BIGINT DEFAULT 0, -- Current calls for this period
    period_start TIMESTAMPTZ NOT NULL DEFAULT NOW(), -- When the current period started
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure one budget per org/workspace combination
    UNIQUE(org_id, workspace_id)
);

-- Organization AI rate limits table
CREATE TABLE IF NOT EXISTS org_ai_rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE, -- NULL for org-level, specific UUID for workspace-level
    rate_limit_per_minute INTEGER NOT NULL DEFAULT 100, -- Requests per minute
    rate_limit_per_hour INTEGER, -- Optional: requests per hour
    rate_limit_per_day INTEGER, -- Optional: requests per day
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure one rate limit config per org/workspace combination
    UNIQUE(org_id, workspace_id)
);

-- Organization AI feature toggles table
CREATE TABLE IF NOT EXISTS org_ai_feature_toggles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE, -- NULL for org-level, specific UUID for workspace-level
    feature_name VARCHAR(100) NOT NULL, -- 'mock_generation', 'contract_diff', 'persona_generation', 'free_form_generation', 'debug_analysis'
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure one toggle per feature per org/workspace combination
    UNIQUE(org_id, workspace_id, feature_name)
);

-- Organization AI usage logs table (audit log)
CREATE TABLE IF NOT EXISTS org_ai_usage_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    feature_name VARCHAR(100) NOT NULL, -- Which AI feature was used
    tokens_used INTEGER NOT NULL DEFAULT 0,
    cost_usd DECIMAL(10, 6) NOT NULL DEFAULT 0.0, -- Cost in USD
    request_id VARCHAR(255), -- Optional: request identifier for tracking
    metadata JSONB DEFAULT '{}'::jsonb, -- Additional metadata (model used, provider, etc.)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Index for time-based queries
    INDEX idx_org_ai_usage_logs_created_at (created_at),
    INDEX idx_org_ai_usage_logs_org_workspace (org_id, workspace_id),
    INDEX idx_org_ai_usage_logs_feature (feature_name)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_org_ai_budgets_org ON org_ai_budgets(org_id);
CREATE INDEX IF NOT EXISTS idx_org_ai_budgets_workspace ON org_ai_budgets(workspace_id);
CREATE INDEX IF NOT EXISTS idx_org_ai_budgets_period ON org_ai_budgets(org_id, period_start);

CREATE INDEX IF NOT EXISTS idx_org_ai_rate_limits_org ON org_ai_rate_limits(org_id);
CREATE INDEX IF NOT EXISTS idx_org_ai_rate_limits_workspace ON org_ai_rate_limits(workspace_id);

CREATE INDEX IF NOT EXISTS idx_org_ai_feature_toggles_org ON org_ai_feature_toggles(org_id);
CREATE INDEX IF NOT EXISTS idx_org_ai_feature_toggles_workspace ON org_ai_feature_toggles(workspace_id);
CREATE INDEX IF NOT EXISTS idx_org_ai_feature_toggles_feature ON org_ai_feature_toggles(feature_name);

-- Add triggers for updated_at
CREATE TRIGGER update_org_ai_budgets_updated_at BEFORE UPDATE ON org_ai_budgets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_org_ai_rate_limits_updated_at BEFORE UPDATE ON org_ai_rate_limits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_org_ai_feature_toggles_updated_at BEFORE UPDATE ON org_ai_feature_toggles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add comments for documentation
COMMENT ON TABLE org_ai_budgets IS 'Organization and workspace-level AI usage budgets with period-based tracking';
COMMENT ON TABLE org_ai_rate_limits IS 'Organization and workspace-level AI rate limits (requests per minute/hour/day)';
COMMENT ON TABLE org_ai_feature_toggles IS 'Organization and workspace-level AI feature enable/disable toggles';
COMMENT ON TABLE org_ai_usage_logs IS 'Audit log of all AI usage for billing, analytics, and compliance';

COMMENT ON COLUMN org_ai_budgets.workspace_id IS 'NULL for org-level budget, specific UUID for workspace-level override';
COMMENT ON COLUMN org_ai_rate_limits.workspace_id IS 'NULL for org-level rate limit, specific UUID for workspace-level override';
COMMENT ON COLUMN org_ai_feature_toggles.workspace_id IS 'NULL for org-level feature toggle, specific UUID for workspace-level override';

