-- Mirror of 20250101000034_plugin_sbom.sql for SQLite backends.
-- SBOM content lives in a nullable TEXT column; the Rust layer
-- serializes/parses JSON at the boundary.

ALTER TABLE plugin_versions ADD COLUMN sbom_json TEXT;
