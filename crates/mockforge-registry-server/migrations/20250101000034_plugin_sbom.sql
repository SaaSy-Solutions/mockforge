-- Add an optional SBOM column to plugin versions.
--
-- An SBOM (Software Bill of Materials — typically CycloneDX JSON) declares
-- the dependency graph the plugin was built from. Storing it at publish
-- time lets a dependency-vulnerability scanner cross-reference known-bad
-- packages without re-parsing the WASM artifact.
--
-- Nullable so existing plugins continue to round-trip cleanly; a missing
-- SBOM surfaces as an informational finding ("no SBOM, skipping vuln
-- checks") rather than a failure.

ALTER TABLE plugin_versions
    ADD COLUMN IF NOT EXISTS sbom_json JSONB;

CREATE INDEX IF NOT EXISTS plugin_versions_sbom_components_gin
    ON plugin_versions USING GIN ((sbom_json -> 'components'));
