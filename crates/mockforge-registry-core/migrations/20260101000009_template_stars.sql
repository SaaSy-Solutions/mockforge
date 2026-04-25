-- SQLite mirror of 20250101000035_template_stars.sql.
--
-- Star counts are always recomputed from this table at read time; no
-- denormalization into templates.stats_json so toggling doesn't have
-- to take a write lock on the parent template.

CREATE TABLE IF NOT EXISTS template_stars (
    template_id TEXT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (template_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_template_stars_user ON template_stars(user_id);
CREATE INDEX IF NOT EXISTS idx_template_stars_template ON template_stars(template_id);
