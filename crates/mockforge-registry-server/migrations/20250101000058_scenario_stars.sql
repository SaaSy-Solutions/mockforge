-- Scenario stars: per-user favorite marking for marketplace scenarios.
-- Mirrors template_stars (20250101000037). Star counts are always computed
-- from this table at read time; we deliberately do not denormalize into
-- scenarios.* so toggling a star doesn't take a row lock on the parent
-- scenario and make popular scenarios a write-contention hotspot.

CREATE TABLE IF NOT EXISTS scenario_stars (
    scenario_id UUID NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (scenario_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_scenario_stars_user ON scenario_stars(user_id);
CREATE INDEX IF NOT EXISTS idx_scenario_stars_scenario ON scenario_stars(scenario_id);
