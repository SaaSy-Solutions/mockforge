-- Multi-tenancy schema for MockForge Cloud
-- Adds organizations, org members, projects, and links existing tables to orgs

-- Organizations table
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) UNIQUE NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    plan VARCHAR(50) NOT NULL DEFAULT 'free' CHECK (plan IN ('free', 'pro', 'team')),
    limits_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    stripe_customer_id VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Organization members table
CREATE TABLE org_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'member')),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(org_id, user_id)
);

-- Projects table (scoped to organizations)
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    slug VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    visibility VARCHAR(50) NOT NULL DEFAULT 'private' CHECK (visibility IN ('private', 'public')),
    default_env VARCHAR(50) NOT NULL DEFAULT 'prod',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(org_id, slug)
);

-- Add org_id to plugins table (nullable for backward compatibility)
ALTER TABLE plugins ADD COLUMN org_id UUID REFERENCES organizations(id) ON DELETE SET NULL;

-- Add org_id to projects for scenarios/templates (if needed later)
-- This is a placeholder for future scenario/template marketplace integration

-- Create indexes for performance
CREATE INDEX idx_organizations_owner ON organizations(owner_id);
CREATE INDEX idx_organizations_slug ON organizations(slug);
CREATE INDEX idx_organizations_plan ON organizations(plan);
CREATE INDEX idx_org_members_org ON org_members(org_id);
CREATE INDEX idx_org_members_user ON org_members(user_id);
CREATE INDEX idx_projects_org ON projects(org_id);
CREATE INDEX idx_projects_slug ON projects(org_id, slug);
CREATE INDEX idx_plugins_org ON plugins(org_id);

-- Add trigger for organizations updated_at
CREATE TRIGGER update_organizations_updated_at BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_org_members_updated_at BEFORE UPDATE ON org_members
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_projects_updated_at BEFORE UPDATE ON projects
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Migration: Create default personal org for existing users
-- This ensures backward compatibility - each user gets their own org
-- Note: This runs only if organizations table is empty (first migration)
-- For existing deployments, users will get orgs on first login
INSERT INTO organizations (id, name, slug, owner_id, plan)
SELECT
    gen_random_uuid(),
    username || '''s Organization',
    'org-' || LOWER(REPLACE(REGEXP_REPLACE(username, '[^a-zA-Z0-9]', '-', 'g'), '--', '-')),
    id,
    'free'
FROM users
WHERE NOT EXISTS (SELECT 1 FROM organizations WHERE owner_id = users.id)
ON CONFLICT (slug) DO NOTHING;

-- Link existing plugins to owner's org (if org exists)
UPDATE plugins p
SET org_id = o.id
FROM organizations o
WHERE p.author_id = o.owner_id
AND p.org_id IS NULL;

-- Create default org membership for owners
INSERT INTO org_members (org_id, user_id, role)
SELECT id, owner_id, 'owner'
FROM organizations
ON CONFLICT DO NOTHING;
