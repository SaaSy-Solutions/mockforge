-- Federation table for composing multiple workspaces

-- Add federation-related enum values
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'federation_create';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'federation_update';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'federation_delete';

ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'federation_created';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'federation_updated';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'federation_deleted';

CREATE TABLE IF NOT EXISTS federations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    services JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(org_id, name)
);

CREATE INDEX IF NOT EXISTS idx_federations_org ON federations(org_id);
CREATE INDEX IF NOT EXISTS idx_federations_created_by ON federations(created_by);
CREATE INDEX IF NOT EXISTS idx_federations_org_created ON federations(org_id, created_at DESC);
