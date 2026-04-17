-- Add language metadata to plugins and a persistent security scan table.
--
-- `language` is source-language of the plugin (rust/python/javascript/etc.) —
-- used to drive the language filter on the registry UI. Default 'rust' keeps
-- existing rows backward-compatible since the Rust SDK is the only published
-- surface so far.
--
-- `plugin_security_scans` holds the latest scan result per plugin version so
-- the /api/v1/plugins/{name}/security endpoint can return persisted data
-- instead of deriving a heuristic at request time.
--
-- This migration is written to be idempotent so re-running it on an
-- environment that has already applied it (or partially applied it) does not
-- fail. sqlx tracks applied migrations by filename, but defensive guards here
-- protect against drift between environments and manual DDL runs.

ALTER TABLE plugins
    ADD COLUMN IF NOT EXISTS language VARCHAR(50) NOT NULL DEFAULT 'rust';

CREATE INDEX IF NOT EXISTS plugins_language_idx ON plugins(language);

CREATE TABLE IF NOT EXISTS plugin_security_scans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plugin_version_id UUID NOT NULL REFERENCES plugin_versions(id) ON DELETE CASCADE,
    status VARCHAR(16) NOT NULL,
    score SMALLINT NOT NULL,
    findings JSONB NOT NULL DEFAULT '[]'::jsonb,
    scanner_version VARCHAR(50),
    scanned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT security_scan_status_values CHECK (status IN ('pass', 'warning', 'fail', 'pending')),
    CONSTRAINT security_scan_score_range CHECK (score >= 0 AND score <= 100)
);

-- A plugin version should have only one scan row (overwritten when re-scanned).
CREATE UNIQUE INDEX IF NOT EXISTS plugin_security_scans_version_idx
    ON plugin_security_scans(plugin_version_id);

CREATE INDEX IF NOT EXISTS plugin_security_scans_scanned_at_idx
    ON plugin_security_scans(scanned_at DESC);
