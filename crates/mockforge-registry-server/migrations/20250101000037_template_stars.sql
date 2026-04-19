-- Template stars: per-user favorite marking for marketplace templates.
-- Counts are always computed from this table at read time; we deliberately
-- do not denormalize into templates.stats_json so toggling a star doesn't
-- take a row lock on the parent template.

CREATE TABLE IF NOT EXISTS template_stars (
    template_id UUID NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (template_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_template_stars_user ON template_stars(user_id);
CREATE INDEX IF NOT EXISTS idx_template_stars_template ON template_stars(template_id);
