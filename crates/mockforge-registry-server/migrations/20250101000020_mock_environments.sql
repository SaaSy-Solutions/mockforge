-- Mock Environments schema
-- Enables environment-specific configurations (dev/test/prod) per workspace
-- Each environment can have its own reality settings, chaos profiles, and drift budgets

-- Mock environments table
CREATE TABLE IF NOT EXISTS mock_environments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL, -- References workspace (may be in collab DB or registry)
    name VARCHAR(50) NOT NULL CHECK (name IN ('dev', 'test', 'prod')),
    reality_config JSONB DEFAULT '{}'::jsonb, -- Environment-specific reality level and config
    chaos_config JSONB DEFAULT '{}'::jsonb, -- Environment-specific chaos engineering config
    drift_budget_config JSONB DEFAULT '{}'::jsonb, -- Environment-specific drift budget config
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, name) -- One environment of each type per workspace
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_mock_environments_workspace ON mock_environments(workspace_id);
CREATE INDEX IF NOT EXISTS idx_mock_environments_name ON mock_environments(name);
CREATE INDEX IF NOT EXISTS idx_mock_environments_workspace_name ON mock_environments(workspace_id, name);

-- Add trigger for updated_at
CREATE TRIGGER update_mock_environments_updated_at BEFORE UPDATE ON mock_environments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add comment for documentation
COMMENT ON TABLE mock_environments IS 'Mock environments (dev/test/prod) with environment-specific configurations for reality, chaos, and drift budgets';
COMMENT ON COLUMN mock_environments.workspace_id IS 'Reference to workspace (UUID, may reference workspaces in collab DB or registry)';
COMMENT ON COLUMN mock_environments.reality_config IS 'Environment-specific reality level and configuration (JSONB)';
COMMENT ON COLUMN mock_environments.chaos_config IS 'Environment-specific chaos engineering configuration (JSONB)';
COMMENT ON COLUMN mock_environments.drift_budget_config IS 'Environment-specific drift budget configuration (JSONB)';
