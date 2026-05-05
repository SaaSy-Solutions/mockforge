-- Cloud Plugins Phase 1 schema
-- Companion to docs/plugins/security/cloud-trust-permissions-rfc.md
--
-- Three pieces:
--   1. organization_trust_roots — per-org Ed25519 keys that authorize
--      org-private plugin signing (RFC §7.1, two-tier trust).
--   2. hosted_mock_plugins — N:M between hosted_mocks and plugin_versions
--      with the per-attachment grant payload (RFC §4.2). The
--      permissions_json column is the structured grant; default is
--      deny-all-everywhere.
--   3. New feature_type values for metering (plugin_attach is a
--      cheap one-shot event; plugin_invoke_ms accumulates wall-time
--      across invocations, populated by the OTLP pipeline in Phase 2).
--      Plus new audit_event_type values for the kill-switch flow
--      (RFC §8.3).
--
-- Plan limits live on organizations.limits_json — no schema change
-- needed there; the cloud_plugins handler reads/writes the new keys
-- (max_plugins_per_mock, max_plugin_invoke_ms_per_month,
-- max_plugin_memory_mb).

-- ─── 1. Organization trust roots ────────────────────────────────────

CREATE TABLE organization_trust_roots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- Ed25519 public key (32 bytes). Stored raw; we don't reuse the
    -- existing user_public_keys table because trust-roots authorize at
    -- the *org* level, not on behalf of a specific user.
    public_key BYTEA NOT NULL,
    -- Human-friendly label shown in the org settings UI ("CI signing
    -- key", "Security team root", etc.).
    name VARCHAR(128) NOT NULL,
    -- Set when the org admin revokes the key. Revoked roots reject
    -- new attaches immediately and existing running plugins fail
    -- re-verification on next boot.
    revoked_at TIMESTAMPTZ,
    revoked_reason TEXT,
    -- Audit attribution for revoke action.
    revoked_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Audit attribution for create action. Nullable because a future
    -- bootstrap path (e.g. SSO-provisioned key) may not have a user.
    created_by UUID REFERENCES users(id) ON DELETE SET NULL
);

-- Per-org listing.
CREATE INDEX idx_org_trust_roots_org ON organization_trust_roots(org_id);
-- Active-only listing skipping revoked roots.
CREATE INDEX idx_org_trust_roots_active
    ON organization_trust_roots(org_id)
    WHERE revoked_at IS NULL;

-- ─── 2. Hosted-mock plugin attachments ──────────────────────────────

CREATE TABLE hosted_mock_plugins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    deployment_id UUID NOT NULL REFERENCES hosted_mocks(id) ON DELETE CASCADE,
    plugin_id UUID NOT NULL REFERENCES plugins(id) ON DELETE RESTRICT,
    -- Pinned version. Migrating to a new version requires an explicit
    -- attach call (and re-grants permissions) — version drift cannot
    -- happen silently.
    plugin_version_id UUID NOT NULL REFERENCES plugin_versions(id) ON DELETE RESTRICT,
    -- Plugin-specific runtime config (the publisher's `ConfigSchema`
    -- as JSON). Distinct from permissions_json: this is "what does
    -- the plugin do," that is "what is it allowed to do."
    config_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Permission grants from the trust RFC §4.2. Default is
    -- deny-all (empty object); admin must explicitly opt in to each
    -- capability the manifest claims.
    permissions_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Soft toggle. Detach is a separate hard action (DELETE row).
    -- Disabled rows stay in the table so the audit trail is preserved.
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    attached_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Most recent enable/disable/permission change. Updates trigger
    -- a manifest reload on the plugin-host.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Audit attribution.
    attached_by UUID REFERENCES users(id) ON DELETE SET NULL,
    -- One plugin can only be attached once per deployment. Re-attach
    -- of the same plugin = update the row.
    UNIQUE (deployment_id, plugin_id)
);

CREATE INDEX idx_hosted_mock_plugins_deployment
    ON hosted_mock_plugins(deployment_id);
CREATE INDEX idx_hosted_mock_plugins_plugin
    ON hosted_mock_plugins(plugin_id);
-- Enabled-only listing — the plugin-host's manifest fetch hits this
-- path on boot, so it gets its own partial index.
CREATE INDEX idx_hosted_mock_plugins_enabled
    ON hosted_mock_plugins(deployment_id)
    WHERE enabled = TRUE;

-- Trigger to bump updated_at on row mutation.
CREATE TRIGGER update_hosted_mock_plugins_updated_at
    BEFORE UPDATE ON hosted_mock_plugins
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ─── 3. New feature_type + audit_event_type values ──────────────────

-- Cheap one-shot event each time a plugin is attached/detached. Used
-- by the go/no-go review and for org-level usage display.
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'plugin_attach';
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'plugin_detach';

-- Wall-time accumulator. Populated by the OTLP pipeline in Phase 2
-- (the metrics bus added in mockforge-plugin-loader/invocation_metrics
-- emits this as `wall_time_us`; cloud aggregator buckets to ms and
-- writes one row per (org, period_start) here).
ALTER TYPE feature_type ADD VALUE IF NOT EXISTS 'plugin_invoke_ms';

-- Kill-switch + revocation audit (RFC §8.3).
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'plugin_attached';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'plugin_detached';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'plugin_revoked';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'plugin_blocklist_hit';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'org_trust_root_created';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'org_trust_root_revoked';
