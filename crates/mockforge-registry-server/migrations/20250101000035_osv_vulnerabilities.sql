-- Persistent cache of OSV-format vulnerability advisories.
--
-- Populated by the `osv_sync` background worker from a configurable source
-- (OSV.dev REST API by default; a filesystem path during dev / air-gapped
-- deploys). Consulted by the plugin security scanner worker when it runs
-- its SBOM-dependency check — an entry here means "if an SBOM references
-- this (ecosystem, name, version), surface the advisory as a finding."
--
-- Schema notes:
--
-- * `advisory_id` is the upstream identifier (e.g. `GHSA-xyz-...`, `CVE-...`,
--   `OSV-2023-1234`). Uniqueness is enforced so re-importing the same
--   advisory doesn't duplicate rows; the sync worker upserts.
-- * `affected_versions` holds the advisory's `affected[].ranges` in OSV
--   format so the matcher can apply SemVer comparisons at query time
--   rather than flattening to a prefix list at import time.
-- * `severity` is a coarse bucket (`critical` / `high` / `medium` / `low`)
--   derived from CVSS where present. Exact scores stay in `extra_json`.

CREATE TABLE IF NOT EXISTS osv_vulnerabilities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    advisory_id VARCHAR(128) NOT NULL,
    ecosystem VARCHAR(50) NOT NULL,
    package_name VARCHAR(255) NOT NULL,
    severity VARCHAR(16) NOT NULL,
    summary TEXT NOT NULL,
    affected_versions JSONB NOT NULL DEFAULT '[]'::jsonb,
    extra_json JSONB,
    modified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    imported_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT osv_severity_values CHECK (severity IN ('critical', 'high', 'medium', 'low')),
    UNIQUE (advisory_id, ecosystem, package_name)
);

CREATE INDEX IF NOT EXISTS osv_vulnerabilities_lookup_idx
    ON osv_vulnerabilities(ecosystem, package_name);

CREATE INDEX IF NOT EXISTS osv_vulnerabilities_modified_idx
    ON osv_vulnerabilities(modified_at DESC);
