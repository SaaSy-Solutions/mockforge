-- Workspace environments and per-environment variables for the cloud
-- workspaces system. The pre-existing `mock_environments` table is constrained
-- to dev/test/prod and is consumed by the scenario-promotion workflow; this
-- table powers the Postman-style environment manager surfaced in the UI.
--
-- One global environment is auto-created per workspace via
-- `is_global = true`. Exactly one row per workspace may have `is_active = true`
-- (enforced by partial unique index).

CREATE TABLE IF NOT EXISTS workspace_environments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    color JSONB,                      -- { hex: "#RRGGBB", name: "Blue" } or NULL
    is_global BOOLEAN NOT NULL DEFAULT false,
    is_active BOOLEAN NOT NULL DEFAULT false,
    display_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workspace_id, name)
);

CREATE INDEX IF NOT EXISTS idx_workspace_environments_workspace
    ON workspace_environments(workspace_id);

-- At most one active environment per workspace
CREATE UNIQUE INDEX IF NOT EXISTS idx_workspace_environments_one_active
    ON workspace_environments(workspace_id) WHERE is_active = true;

-- At most one global environment per workspace
CREATE UNIQUE INDEX IF NOT EXISTS idx_workspace_environments_one_global
    ON workspace_environments(workspace_id) WHERE is_global = true;

CREATE TRIGGER update_workspace_environments_updated_at
    BEFORE UPDATE ON workspace_environments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS workspace_environment_variables (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    environment_id UUID NOT NULL REFERENCES workspace_environments(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    value TEXT NOT NULL DEFAULT '',
    is_secret BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (environment_id, name)
);

CREATE INDEX IF NOT EXISTS idx_workspace_environment_variables_env
    ON workspace_environment_variables(environment_id);

CREATE TRIGGER update_workspace_environment_variables_updated_at
    BEFORE UPDATE ON workspace_environment_variables
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE workspace_environments IS
    'Postman-style environments per cloud workspace. Distinct from `mock_environments` (dev/test/prod).';
COMMENT ON COLUMN workspace_environments.is_global IS
    'Workspace-wide default environment, auto-created on first list and undeletable.';
