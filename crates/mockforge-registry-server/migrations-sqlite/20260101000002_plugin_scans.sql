-- Minimal marketplace tables for the OSS admin SQLite backend.
--
-- The full marketplace schema (reviews, tags, scenarios, templates, federations)
-- is Postgres-only — see `migrations/20250101000001_init.sql`. This migration
-- introduces just enough surface area for the plugin security scanner to
-- operate against a SQLite-backed instance:
--
--   * `plugins` — one row per uniquely-named plugin.
--   * `plugin_versions` — published artifacts. `file_size` + `checksum` are
--     what the scanner compares against when validating a download.
--   * `plugin_security_scans` — persisted scan verdicts, keyed by version
--     with at-most-one row per version (latest wins).
--
-- Cross-reference: kept deliberately compatible with the Postgres schema
-- shape so the same handler/store code compiles for both backends. UUIDs,
-- timestamps, and JSON follow the translation table documented in
-- `20260101000001_init.sql`.

CREATE TABLE IF NOT EXISTS plugins (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    description TEXT NOT NULL,
    current_version TEXT NOT NULL,
    category TEXT NOT NULL,
    license TEXT NOT NULL,
    repository TEXT,
    homepage TEXT,
    downloads_total INTEGER NOT NULL DEFAULT 0,
    rating_avg REAL NOT NULL DEFAULT 0,
    rating_count INTEGER NOT NULL DEFAULT 0,
    author_id TEXT NOT NULL,
    verified_at TEXT,
    language TEXT NOT NULL DEFAULT 'rust',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_plugins_language ON plugins(language);
CREATE INDEX IF NOT EXISTS idx_plugins_author ON plugins(author_id);

CREATE TABLE IF NOT EXISTS plugin_versions (
    id TEXT PRIMARY KEY NOT NULL,
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    download_url TEXT NOT NULL,
    checksum TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    min_mockforge_version TEXT,
    yanked BOOLEAN NOT NULL DEFAULT 0,
    downloads INTEGER NOT NULL DEFAULT 0,
    published_at TEXT NOT NULL DEFAULT (datetime('now')),

    UNIQUE(plugin_id, version)
);

CREATE INDEX IF NOT EXISTS idx_plugin_versions_plugin_id ON plugin_versions(plugin_id);

CREATE TABLE IF NOT EXISTS plugin_security_scans (
    id TEXT PRIMARY KEY NOT NULL,
    plugin_version_id TEXT NOT NULL REFERENCES plugin_versions(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    score INTEGER NOT NULL,
    findings TEXT NOT NULL DEFAULT '[]',
    scanner_version TEXT,
    scanned_at TEXT NOT NULL DEFAULT (datetime('now')),

    CHECK (status IN ('pass', 'warning', 'fail', 'pending')),
    CHECK (score >= 0 AND score <= 100)
);

-- One scan row per version; re-scanning overwrites via ON CONFLICT.
CREATE UNIQUE INDEX IF NOT EXISTS idx_plugin_security_scans_version
    ON plugin_security_scans(plugin_version_id);
CREATE INDEX IF NOT EXISTS idx_plugin_security_scans_scanned_at
    ON plugin_security_scans(scanned_at DESC);
