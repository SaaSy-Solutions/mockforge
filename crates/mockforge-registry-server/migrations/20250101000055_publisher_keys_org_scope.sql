-- Org-scoped publisher attestation keys.
--
-- A publisher key has been user-scoped since the original SBOM
-- attestation work (migration 20250101000036). That fits a single-author
-- workflow but breaks down for teams: a CI bot signing on behalf of an
-- organization, or shared on-call rotation, has no way to publish using
-- a key that other teammates can audit and revoke.
--
-- We add a nullable `org_id` so a key can either be:
--   * personal (`org_id IS NULL`) — same as today; the verifier still
--     accepts it for the owner's plugin publishes.
--   * org-tagged — visible to org Owners/Admins via the new
--     `/api/v1/organizations/{org_id}/public-keys` endpoint and accepted
--     by the verifier when *any* member of that org publishes.
--
-- ON DELETE SET NULL because deleting an org should not silently delete
-- a publisher's history of signatures. The key falls back to a personal
-- key in that case, which keeps `plugin_versions.sbom_signed_key_id`
-- references intact.
--
-- A new audit-event variant for create/revoke/rotate ships in the same
-- migration so handlers can record under a single CHECK type.

ALTER TABLE user_public_keys
    ADD COLUMN IF NOT EXISTS org_id UUID
        REFERENCES organizations(id) ON DELETE SET NULL;

-- Partial index on org_id IS NOT NULL so the org-scoped list endpoint
-- doesn't scan personal keys.
CREATE INDEX IF NOT EXISTS user_public_keys_org_idx
    ON user_public_keys(org_id)
    WHERE org_id IS NOT NULL AND revoked_at IS NULL;

-- Audit-event variants for the publisher-key lifecycle. ALTER TYPE ADD
-- VALUE is the standard pattern across this codebase
-- (see 20250101000029_federations.sql).
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'publisher_key_created';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'publisher_key_revoked';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'publisher_key_rotated';
