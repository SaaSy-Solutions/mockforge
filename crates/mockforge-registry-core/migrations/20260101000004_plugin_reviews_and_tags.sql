-- SQLite marketplace: reviews + tags for plugins.
--
-- Second pass of the "bring marketplace to OSS mode" work. The first pass
-- (20260101000002_plugin_scans.sql) added the tables needed by the security
-- scanner. This one adds the reviews + tags surface so the OSS registry UI
-- can render stars, review counts, and filter by tag without falling back
-- to empty stubs.
--
-- Scenarios, templates, and federations remain Postgres-only — they're
-- multi-tenant SaaS features that the single-tenant OSS admin doesn't need.

CREATE TABLE IF NOT EXISTS reviews (
    id TEXT PRIMARY KEY NOT NULL,
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating INTEGER NOT NULL,
    title TEXT,
    comment TEXT NOT NULL,
    helpful_count INTEGER NOT NULL DEFAULT 0,
    unhelpful_count INTEGER NOT NULL DEFAULT 0,
    verified BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    CHECK (rating >= 1 AND rating <= 5),
    UNIQUE(plugin_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_reviews_plugin_id ON reviews(plugin_id);
CREATE INDEX IF NOT EXISTS idx_reviews_user_id ON reviews(user_id);
-- Ranking query: most-helpful-first with created_at as tie-breaker. Aligns
-- with the Postgres get_by_plugin ORDER BY.
CREATE INDEX IF NOT EXISTS idx_reviews_ranking
    ON reviews(plugin_id, helpful_count DESC, created_at DESC);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS plugin_tags (
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (plugin_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_plugin_tags_tag_id ON plugin_tags(tag_id);
