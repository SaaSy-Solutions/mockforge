-- Organization Templates schema
-- Enables org admins to define standard blueprints and security baseline configs
-- for workspace creation, allowing teams to start from templates rather than scratch

-- Organization templates table
CREATE TABLE IF NOT EXISTS org_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    blueprint_config JSONB DEFAULT '{}'::jsonb, -- Blueprint configuration (personas, reality defaults, flows, etc.)
    security_baseline JSONB DEFAULT '{}'::jsonb, -- Security baseline configuration (RBAC defaults, validation modes, etc.)
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    is_default BOOLEAN DEFAULT FALSE, -- Whether this is the default template for the org
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure unique template names per org
    UNIQUE(org_id, name)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_org_templates_org ON org_templates(org_id);
CREATE INDEX IF NOT EXISTS idx_org_templates_created_by ON org_templates(created_by);
CREATE INDEX IF NOT EXISTS idx_org_templates_default ON org_templates(org_id, is_default) WHERE is_default = TRUE;

-- Add trigger for updated_at
CREATE TRIGGER update_org_templates_updated_at BEFORE UPDATE ON org_templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add comments for documentation
COMMENT ON TABLE org_templates IS 'Organization-level templates for workspace creation with blueprint and security baseline configs';
COMMENT ON COLUMN org_templates.blueprint_config IS 'Blueprint configuration including personas, reality defaults, sample flows, and playground collections';
COMMENT ON COLUMN org_templates.security_baseline IS 'Security baseline configuration including RBAC defaults, validation modes, and security policies';
COMMENT ON COLUMN org_templates.is_default IS 'Whether this template is used by default when creating new workspaces in the org';
