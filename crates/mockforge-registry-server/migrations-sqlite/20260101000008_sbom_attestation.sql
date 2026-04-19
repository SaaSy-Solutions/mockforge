-- SQLite mirror of 20250101000036_sbom_attestation.sql.

CREATE TABLE IF NOT EXISTS user_public_keys (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    algorithm TEXT NOT NULL DEFAULT 'ed25519',
    public_key_b64 TEXT NOT NULL,
    label TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    revoked_at TEXT,

    CHECK (algorithm IN ('ed25519'))
);

CREATE INDEX IF NOT EXISTS idx_user_public_keys_user
    ON user_public_keys(user_id);

ALTER TABLE plugin_versions ADD COLUMN sbom_signed_key_id TEXT
    REFERENCES user_public_keys(id) ON DELETE SET NULL;

ALTER TABLE plugin_versions ADD COLUMN sbom_signed_at TEXT;
