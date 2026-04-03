-- Cloud services and fixtures tables

-- Service enum values
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'service_create';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'service_update';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'service_delete';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'fixture_create';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'fixture_update';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'fixture_delete';

ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'service_created';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'service_updated';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'service_deleted';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'fixture_created';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'fixture_updated';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'fixture_deleted';

-- Services table: mock API service definitions
CREATE TABLE IF NOT EXISTS services (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    base_url TEXT NOT NULL DEFAULT '',
    enabled BOOLEAN NOT NULL DEFAULT true,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    routes JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_services_org ON services(org_id);
CREATE INDEX IF NOT EXISTS idx_services_workspace ON services(workspace_id);
CREATE INDEX IF NOT EXISTS idx_services_org_enabled ON services(org_id) WHERE enabled = true;

-- Fixtures table: mock response fixture definitions
CREATE TABLE IF NOT EXISTS fixtures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    path TEXT NOT NULL DEFAULT '',
    method VARCHAR(10) NOT NULL DEFAULT 'GET',
    content JSONB,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    route_path TEXT,
    protocol VARCHAR(50),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fixtures_org ON fixtures(org_id);
CREATE INDEX IF NOT EXISTS idx_fixtures_workspace ON fixtures(workspace_id);
CREATE INDEX IF NOT EXISTS idx_fixtures_org_method ON fixtures(org_id, method);
