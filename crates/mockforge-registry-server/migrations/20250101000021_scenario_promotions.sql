-- Scenario Promotions schema
-- Tracks scenario promotions between environments (dev → test → prod)
-- Includes approval workflow support for high-impact changes

-- Scenario promotions table
CREATE TABLE IF NOT EXISTS scenario_promotions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scenario_id UUID NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    scenario_version VARCHAR(50) NOT NULL, -- Version being promoted
    workspace_id UUID NOT NULL, -- Workspace where promotion occurs
    from_environment VARCHAR(50) NOT NULL CHECK (from_environment IN ('dev', 'test', 'prod')),
    to_environment VARCHAR(50) NOT NULL CHECK (to_environment IN ('dev', 'test', 'prod')),
    promoted_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    approved_by UUID REFERENCES users(id) ON DELETE SET NULL, -- Nullable for auto-approved promotions
    status VARCHAR(50) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'completed', 'failed')),
    requires_approval BOOLEAN NOT NULL DEFAULT FALSE, -- Whether this promotion requires approval
    approval_required_reason TEXT, -- Reason why approval is required (e.g., "high-impact", "auth", "billing")
    comments TEXT, -- Comments from promoter
    approval_comments TEXT, -- Comments from approver
    completed_at TIMESTAMPTZ, -- When promotion was completed
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure valid promotion path (dev → test → prod)
    CONSTRAINT valid_promotion_path CHECK (
        (from_environment = 'dev' AND to_environment = 'test') OR
        (from_environment = 'test' AND to_environment = 'prod')
    )
);

-- Scenario environment versions table
-- Tracks which scenario version is active in each environment for each workspace
CREATE TABLE IF NOT EXISTS scenario_environment_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scenario_id UUID NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL,
    environment VARCHAR(50) NOT NULL CHECK (environment IN ('dev', 'test', 'prod')),
    scenario_version VARCHAR(50) NOT NULL, -- Active version in this environment
    promoted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    promoted_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    promotion_id UUID REFERENCES scenario_promotions(id) ON DELETE SET NULL, -- Link to promotion record
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- One active version per scenario per workspace per environment
    UNIQUE(scenario_id, workspace_id, environment)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_scenario_promotions_scenario ON scenario_promotions(scenario_id);
CREATE INDEX IF NOT EXISTS idx_scenario_promotions_workspace ON scenario_promotions(workspace_id);
CREATE INDEX IF NOT EXISTS idx_scenario_promotions_status ON scenario_promotions(status);
CREATE INDEX IF NOT EXISTS idx_scenario_promotions_promoted_by ON scenario_promotions(promoted_by);
CREATE INDEX IF NOT EXISTS idx_scenario_promotions_approved_by ON scenario_promotions(approved_by);
CREATE INDEX IF NOT EXISTS idx_scenario_promotions_created ON scenario_promotions(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_scenario_env_versions_scenario ON scenario_environment_versions(scenario_id);
CREATE INDEX IF NOT EXISTS idx_scenario_env_versions_workspace ON scenario_environment_versions(workspace_id);
CREATE INDEX IF NOT EXISTS idx_scenario_env_versions_env ON scenario_environment_versions(environment);
CREATE INDEX IF NOT EXISTS idx_scenario_env_versions_workspace_env ON scenario_environment_versions(workspace_id, environment);

-- Add triggers for updated_at
CREATE TRIGGER update_scenario_promotions_updated_at BEFORE UPDATE ON scenario_promotions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_scenario_env_versions_updated_at BEFORE UPDATE ON scenario_environment_versions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add comments for documentation
COMMENT ON TABLE scenario_promotions IS 'Tracks scenario promotions between environments with approval workflow support';
COMMENT ON TABLE scenario_environment_versions IS 'Tracks active scenario version per environment per workspace';
COMMENT ON COLUMN scenario_promotions.requires_approval IS 'Whether this promotion requires approval (e.g., for high-impact scenarios)';
COMMENT ON COLUMN scenario_promotions.approval_required_reason IS 'Reason why approval is required (e.g., "high-impact", "auth", "billing")';
