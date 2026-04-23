-- SQLite mirror of the Postgres templates + scenarios marketplace schema.
--
-- Why this exists: the OSS admin UI surfaces TemplateMarketplacePage and
-- scenario handlers against the same `RegistryStore` trait the SaaS binary
-- uses. Without these tables the trait methods stub to empty, and every
-- marketplace-adjacent page renders blank on a SQLite-backed install.
--
-- Scope matches the Postgres schema in
-- `migrations/20250101000008_templates_scenarios.sql` — table shapes are
-- preserved so the same domain code can drive both backends. Types get
-- the usual SQLite translation (TEXT for UUID/JSON/TIMESTAMPTZ, INTEGER
-- for BOOLEAN/SERIAL). Postgres-specific features (TEXT[], GIN FTS
-- indexes, enum alters, triggers) are dropped — the OSS admin is
-- single-tenant and doesn't need org-level full-text search.
--
-- Store methods for these tables are still being ported; this migration
-- is the first half so the tables exist and trait implementations can
-- land incrementally.

CREATE TABLE IF NOT EXISTS templates (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT NOT NULL,
    author_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    category TEXT NOT NULL,
    -- `tags` and `requirements` are TEXT[] in Postgres; store as JSON arrays here.
    tags TEXT NOT NULL DEFAULT '[]',
    content_json TEXT NOT NULL,
    readme TEXT,
    example_usage TEXT,
    requirements TEXT NOT NULL DEFAULT '[]',
    compatibility_json TEXT NOT NULL DEFAULT '{}',
    stats_json TEXT NOT NULL DEFAULT
        '{"downloads":0,"stars":0,"forks":0,"rating":0.0,"rating_count":0}',
    published BOOLEAN NOT NULL DEFAULT 0,
    verified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    CHECK (category IN (
        'network-chaos', 'service-failure', 'load-testing',
        'resilience-testing', 'security-testing', 'data-corruption',
        'multi-protocol', 'custom-scenario'
    )),
    UNIQUE(name, version)
);

CREATE INDEX IF NOT EXISTS idx_templates_org ON templates(org_id);
CREATE INDEX IF NOT EXISTS idx_templates_author ON templates(author_id);
CREATE INDEX IF NOT EXISTS idx_templates_category ON templates(category);
CREATE INDEX IF NOT EXISTS idx_templates_published ON templates(published);
CREATE INDEX IF NOT EXISTS idx_templates_slug ON templates(slug);

CREATE TABLE IF NOT EXISTS template_versions (
    id TEXT PRIMARY KEY NOT NULL,
    template_id TEXT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    content_json TEXT NOT NULL,
    download_url TEXT,
    checksum TEXT,
    file_size INTEGER NOT NULL DEFAULT 0,
    yanked BOOLEAN NOT NULL DEFAULT 0,
    published_at TEXT NOT NULL DEFAULT (datetime('now')),

    UNIQUE(template_id, version)
);

CREATE INDEX IF NOT EXISTS idx_template_versions_template ON template_versions(template_id);

CREATE TABLE IF NOT EXISTS template_reviews (
    id TEXT PRIMARY KEY NOT NULL,
    template_id TEXT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    reviewer_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating INTEGER NOT NULL,
    title TEXT,
    comment TEXT NOT NULL,
    helpful_count INTEGER NOT NULL DEFAULT 0,
    verified_use BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    CHECK (rating >= 1 AND rating <= 5),
    UNIQUE(template_id, reviewer_id)
);

CREATE INDEX IF NOT EXISTS idx_template_reviews_template ON template_reviews(template_id);
CREATE INDEX IF NOT EXISTS idx_template_reviews_reviewer ON template_reviews(reviewer_id);

CREATE TABLE IF NOT EXISTS scenarios (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT REFERENCES organizations(id) ON DELETE SET NULL,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL,
    description TEXT NOT NULL,
    author_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    current_version TEXT NOT NULL,
    category TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    license TEXT NOT NULL,
    repository TEXT,
    homepage TEXT,
    manifest_json TEXT NOT NULL,
    downloads_total INTEGER NOT NULL DEFAULT 0,
    rating_avg REAL NOT NULL DEFAULT 0,
    rating_count INTEGER NOT NULL DEFAULT 0,
    verified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_scenarios_org ON scenarios(org_id);
CREATE INDEX IF NOT EXISTS idx_scenarios_author ON scenarios(author_id);
CREATE INDEX IF NOT EXISTS idx_scenarios_category ON scenarios(category);
CREATE INDEX IF NOT EXISTS idx_scenarios_slug ON scenarios(slug);

CREATE TABLE IF NOT EXISTS scenario_versions (
    id TEXT PRIMARY KEY NOT NULL,
    scenario_id TEXT NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    download_url TEXT NOT NULL,
    checksum TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    min_mockforge_version TEXT,
    yanked BOOLEAN NOT NULL DEFAULT 0,
    downloads INTEGER NOT NULL DEFAULT 0,
    published_at TEXT NOT NULL DEFAULT (datetime('now')),

    UNIQUE(scenario_id, version)
);

CREATE INDEX IF NOT EXISTS idx_scenario_versions_scenario
    ON scenario_versions(scenario_id);

CREATE TABLE IF NOT EXISTS scenario_reviews (
    id TEXT PRIMARY KEY NOT NULL,
    scenario_id TEXT NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    reviewer_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating INTEGER NOT NULL,
    title TEXT,
    comment TEXT NOT NULL,
    helpful_count INTEGER NOT NULL DEFAULT 0,
    verified_purchase BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    CHECK (rating >= 1 AND rating <= 5),
    UNIQUE(scenario_id, reviewer_id)
);

CREATE INDEX IF NOT EXISTS idx_scenario_reviews_scenario
    ON scenario_reviews(scenario_id);
CREATE INDEX IF NOT EXISTS idx_scenario_reviews_reviewer
    ON scenario_reviews(reviewer_id);

CREATE TABLE IF NOT EXISTS template_tags (
    template_id TEXT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    tag_name TEXT NOT NULL,
    PRIMARY KEY (template_id, tag_name)
);

CREATE TABLE IF NOT EXISTS scenario_tags (
    scenario_id TEXT NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    tag_name TEXT NOT NULL,
    PRIMARY KEY (scenario_id, tag_name)
);
