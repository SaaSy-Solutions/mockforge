-- Environment permission policies schema
-- Enables environment-scoped RBAC (e.g., "Only Platform can change reality levels in prod")

-- Environment permission policies table
CREATE TABLE IF NOT EXISTS environment_permission_policies (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT, -- Optional: for org-wide policies
    workspace_id TEXT, -- Optional: for workspace-specific policies
    environment TEXT NOT NULL CHECK (environment IN ('dev', 'test', 'prod')),
    permission TEXT NOT NULL, -- Permission name (e.g., 'ManageSettings', 'MockUpdate')
    allowed_roles TEXT NOT NULL, -- JSON array of role names (e.g., '["admin", "platform"]')
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_env_policy_org ON environment_permission_policies(org_id);
CREATE INDEX idx_env_policy_workspace ON environment_permission_policies(workspace_id);
CREATE INDEX idx_env_policy_env ON environment_permission_policies(environment);
CREATE INDEX idx_env_policy_permission ON environment_permission_policies(permission);
CREATE INDEX idx_env_policy_env_perm ON environment_permission_policies(environment, permission);

