-- Org invitations (cloud-enablement task #15 / Phase 1).
--
-- Email-based invitations to join an organization. Pre-existing user
-- management was a local-only page; we're folding it into the cloud
-- OrganizationPage with three new tabs: Members (existing) +
-- Invitations (this) + Activity (separate aggregator endpoint).
--
-- See docs/cloud/CLOUD_USER_MANAGEMENT_CONSOLIDATION.md.

CREATE TABLE IF NOT EXISTS org_invitations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL,                         -- mirrors OrgRole vocabulary
    -- Single-use random token sent in the invitation email. Hashed at
    -- rest like api_tokens; redeemers POST the plaintext token back.
    token_hash TEXT NOT NULL,
    token_prefix TEXT NOT NULL,                 -- short prefix for display/log
    invited_by UUID REFERENCES users(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'pending',     -- 'pending' | 'accepted' | 'cancelled' | 'expired'
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    accepted_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- Per-org list view, pending first.
CREATE INDEX IF NOT EXISTS idx_org_invitations_org
    ON org_invitations(org_id, status, created_at DESC);
-- Email-based lookup so we can dedupe pending invites to the same address.
CREATE UNIQUE INDEX IF NOT EXISTS idx_org_invitations_pending_email
    ON org_invitations(org_id, lower(email))
    WHERE status = 'pending';
-- Token prefix lookup for log filters.
CREATE INDEX IF NOT EXISTS idx_org_invitations_token_prefix
    ON org_invitations(token_prefix)
    WHERE status = 'pending';
