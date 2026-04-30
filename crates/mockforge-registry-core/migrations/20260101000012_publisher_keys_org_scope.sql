-- SQLite mirror of 20250101000055_publisher_keys_org_scope.sql.
--
-- SQLite doesn't have native enums, so the audit_event_type extension
-- is a no-op here — `event_type` is TEXT in this backend. We only need
-- to add the new `org_id` column.

ALTER TABLE user_public_keys ADD COLUMN org_id TEXT
    REFERENCES organizations(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_user_public_keys_org
    ON user_public_keys(org_id);
