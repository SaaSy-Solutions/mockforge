-- Mirror of 20250101000035_osv_vulnerabilities.sql for SQLite backends.
-- JSON columns are TEXT-encoded at the application boundary.

CREATE TABLE IF NOT EXISTS osv_vulnerabilities (
    id TEXT PRIMARY KEY NOT NULL,
    advisory_id TEXT NOT NULL,
    ecosystem TEXT NOT NULL,
    package_name TEXT NOT NULL,
    severity TEXT NOT NULL,
    summary TEXT NOT NULL,
    affected_versions TEXT NOT NULL DEFAULT '[]',
    extra_json TEXT,
    modified_at TEXT NOT NULL DEFAULT (datetime('now')),
    imported_at TEXT NOT NULL DEFAULT (datetime('now')),

    CHECK (severity IN ('critical', 'high', 'medium', 'low')),
    UNIQUE (advisory_id, ecosystem, package_name)
);

CREATE INDEX IF NOT EXISTS idx_osv_vulnerabilities_lookup
    ON osv_vulnerabilities(ecosystem, package_name);

CREATE INDEX IF NOT EXISTS idx_osv_vulnerabilities_modified
    ON osv_vulnerabilities(modified_at DESC);
