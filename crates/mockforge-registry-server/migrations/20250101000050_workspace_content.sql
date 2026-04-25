-- Workspace content: environments, variables, folders, requests.
-- Lets the cloud UI operate on per-workspace content, matching the self-hosted surface
-- exposed under /__mockforge/workspaces/*.

-- Per-org manual sort order. Default keeps existing rows contiguous and sortable.
ALTER TABLE workspaces
    ADD COLUMN IF NOT EXISTS sort_order INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_workspaces_org_sort ON workspaces(org_id, sort_order, created_at);

-- Environments scoped to a workspace (e.g. dev/staging/prod).
CREATE TABLE workspace_environments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    color_hex VARCHAR(16) NOT NULL DEFAULT '',
    color_name VARCHAR(32) NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, name)
);
CREATE INDEX idx_workspace_environments_ws ON workspace_environments(workspace_id, sort_order);

-- Only one active environment per workspace (enforced by app; index supports lookup).
CREATE INDEX idx_workspace_environments_active
    ON workspace_environments(workspace_id)
    WHERE is_active;

-- Variables are keyed by (environment_id, name); value may be marked secret for masking.
CREATE TABLE workspace_env_variables (
    environment_id UUID NOT NULL REFERENCES workspace_environments(id) ON DELETE CASCADE,
    name VARCHAR(256) NOT NULL,
    value TEXT NOT NULL,
    is_secret BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (environment_id, name)
);

-- Folder tree inside a workspace. parent_folder_id is nullable for root folders.
CREATE TABLE workspace_folders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    parent_folder_id UUID REFERENCES workspace_folders(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_workspace_folders_ws ON workspace_folders(workspace_id);
CREATE INDEX idx_workspace_folders_parent ON workspace_folders(parent_folder_id);

-- Mock requests (stored templates, not live proxy).
CREATE TABLE workspace_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    folder_id UUID REFERENCES workspace_folders(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    method VARCHAR(16) NOT NULL,
    path TEXT NOT NULL,
    status_code INTEGER NOT NULL DEFAULT 200,
    response_body TEXT NOT NULL DEFAULT '',
    request_headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    response_headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_workspace_requests_ws ON workspace_requests(workspace_id);
CREATE INDEX idx_workspace_requests_folder ON workspace_requests(folder_id);
