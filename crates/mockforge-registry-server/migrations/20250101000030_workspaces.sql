-- Cloud workspaces table for managing mock API workspace definitions

ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'workspace_create';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'workspace_update';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'workspace_delete';

ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'workspace_created';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'workspace_updated';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'workspace_deleted';

CREATE TABLE IF NOT EXISTS workspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT false,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(org_id, name)
);

CREATE INDEX IF NOT EXISTS idx_workspaces_org ON workspaces(org_id);
CREATE INDEX IF NOT EXISTS idx_workspaces_org_active ON workspaces(org_id) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_workspaces_created_by ON workspaces(created_by);
