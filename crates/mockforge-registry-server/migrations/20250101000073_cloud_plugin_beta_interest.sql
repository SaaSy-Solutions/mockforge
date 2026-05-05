-- Cloud Plugins beta interest signups (Phase 0 demand validation).
--
-- A row is inserted when an authenticated user clicks the "Request beta
-- access" CTA on the cloud plugin-registry page. UNIQUE (user_id) makes
-- repeat submissions an UPSERT — the latest use_case wins. The org_id
-- column captures which org context the user was in at signup time so
-- the go/no-go review can ask "how many distinct orgs (and at which
-- plan tiers) want this".

CREATE TABLE cloud_plugin_beta_interest (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Nullable so a user with no current org context can still register.
    org_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    -- Free-text "what would you build with cloud plugins?". Optional.
    use_case TEXT,
    -- Snapshot of the org plan at signup ('free' | 'pro' | 'team' | NULL).
    -- Stored verbatim to avoid joins during analysis and to preserve the
    -- value even if the org later upgrades.
    plan_at_signup VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id)
);

CREATE INDEX idx_cloud_plugin_beta_interest_org ON cloud_plugin_beta_interest(org_id);
CREATE INDEX idx_cloud_plugin_beta_interest_created
    ON cloud_plugin_beta_interest(created_at DESC);
