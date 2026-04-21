-- SQLite mirror of federation_scenario_activations.
--
-- SQLite stores event/feature types as TEXT so no enum extensions are needed.
-- JSONB becomes TEXT; the application layer serializes JSON in and out.

CREATE TABLE IF NOT EXISTS federation_scenario_activations (
    id TEXT PRIMARY KEY NOT NULL,
    federation_id TEXT NOT NULL REFERENCES federations(id) ON DELETE CASCADE,
    scenario_id TEXT,
    scenario_name TEXT NOT NULL,
    manifest_snapshot TEXT NOT NULL,
    service_overrides TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'deactivated', 'failed')),
    per_service_state TEXT NOT NULL DEFAULT '[]',
    activated_by TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    activated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deactivated_at TEXT
);

-- SQLite supports partial unique indexes; use the same invariant as Postgres.
CREATE UNIQUE INDEX IF NOT EXISTS idx_fed_scenario_activations_one_active
    ON federation_scenario_activations(federation_id)
    WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_fed_scenario_activations_federation
    ON federation_scenario_activations(federation_id, activated_at DESC);

CREATE INDEX IF NOT EXISTS idx_fed_scenario_activations_scenario
    ON federation_scenario_activations(scenario_id)
    WHERE scenario_id IS NOT NULL;
