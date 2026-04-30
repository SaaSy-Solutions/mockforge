-- SQLite mirror of 20250101000057_scenario_stars.sql.
--
-- Star counts are always recomputed from this table at read time; no
-- denormalization into scenarios.* so toggling doesn't have to take a
-- write lock on the parent scenario.

CREATE TABLE IF NOT EXISTS scenario_stars (
    scenario_id TEXT NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (scenario_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_scenario_stars_user ON scenario_stars(user_id);
CREATE INDEX IF NOT EXISTS idx_scenario_stars_scenario ON scenario_stars(scenario_id);
