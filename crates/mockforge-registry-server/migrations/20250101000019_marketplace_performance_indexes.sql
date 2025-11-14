-- Additional performance indexes for marketplace queries
-- This migration adds indexes to optimize common marketplace query patterns

-- ============================================================================
-- Plugin table additional indexes
-- ============================================================================

-- Index for name lookups (used in find_by_name)
CREATE INDEX IF NOT EXISTS idx_plugins_name ON plugins(name);

-- Index for downloads sorting (popular plugins)
CREATE INDEX IF NOT EXISTS idx_plugins_downloads ON plugins(downloads_total DESC);

-- Index for rating sorting (highly rated plugins)
CREATE INDEX IF NOT EXISTS idx_plugins_rating ON plugins(rating_avg DESC) WHERE rating_count > 0;

-- Composite index for name sorting
CREATE INDEX IF NOT EXISTS idx_plugins_name_asc ON plugins(name ASC);

-- ============================================================================
-- Template table additional indexes
-- ============================================================================

-- Composite index for name + version lookups (used in find_by_name_version)
CREATE INDEX IF NOT EXISTS idx_templates_name_version ON templates(name, version);

-- Index for name lookups
CREATE INDEX IF NOT EXISTS idx_templates_name ON templates(name);

-- Index for downloads sorting (popular templates)
CREATE INDEX IF NOT EXISTS idx_templates_downloads ON templates(downloads DESC);

-- Index for rating sorting (highly rated templates)
CREATE INDEX IF NOT EXISTS idx_templates_rating ON templates(rating_avg DESC) WHERE rating_count > 0;

-- Composite index for org + name (common lookup pattern)
CREATE INDEX IF NOT EXISTS idx_templates_org_name ON templates(org_id, name) WHERE org_id IS NOT NULL;

-- ============================================================================
-- Scenario table additional indexes
-- ============================================================================

-- Index for name lookups (used in find_by_name)
CREATE INDEX IF NOT EXISTS idx_scenarios_name ON scenarios(name);

-- Composite index for org + name (common lookup pattern)
CREATE INDEX IF NOT EXISTS idx_scenarios_org_name ON scenarios(org_id, name) WHERE org_id IS NOT NULL;

-- Composite index for public scenarios (org_id IS NULL) - common search pattern
CREATE INDEX IF NOT EXISTS idx_scenarios_public ON scenarios(category, created_at DESC) WHERE org_id IS NULL;

-- Composite index for org scenarios - common search pattern
CREATE INDEX IF NOT EXISTS idx_scenarios_org_public ON scenarios(org_id, category, created_at DESC) WHERE org_id IS NOT NULL;

-- ============================================================================
-- Template versions table additional indexes
-- ============================================================================

-- Composite index for template + version lookups
CREATE INDEX IF NOT EXISTS idx_template_versions_template_version ON template_versions(template_id, version);

-- Index for downloads sorting
CREATE INDEX IF NOT EXISTS idx_template_versions_downloads ON template_versions(downloads DESC);

-- ============================================================================
-- Scenario versions table additional indexes
-- ============================================================================

-- Composite index for scenario + version lookups
CREATE INDEX IF NOT EXISTS idx_scenario_versions_scenario_version ON scenario_versions(scenario_id, version);

-- Index for downloads sorting
CREATE INDEX IF NOT EXISTS idx_scenario_versions_downloads ON scenario_versions(downloads DESC);

-- ============================================================================
-- Review tables additional indexes for sorting
-- ============================================================================

-- Index for helpful reviews (sorting by helpful_count)
CREATE INDEX IF NOT EXISTS idx_reviews_helpful ON reviews(plugin_id, helpful_count DESC);

CREATE INDEX IF NOT EXISTS idx_template_reviews_helpful ON template_reviews(template_id, helpful_count DESC);

CREATE INDEX IF NOT EXISTS idx_scenario_reviews_helpful ON scenario_reviews(scenario_id, helpful_count DESC);

-- Index for recent reviews (sorting by created_at)
CREATE INDEX IF NOT EXISTS idx_reviews_created ON reviews(plugin_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_template_reviews_created ON template_reviews(template_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_scenario_reviews_created ON scenario_reviews(scenario_id, created_at DESC);

-- ============================================================================
-- Plugin versions table additional indexes
-- ============================================================================

-- Composite index for plugin + version lookups
CREATE INDEX IF NOT EXISTS idx_plugin_versions_plugin_version ON plugin_versions(plugin_id, version);

-- Index for downloads sorting
CREATE INDEX IF NOT EXISTS idx_plugin_versions_downloads ON plugin_versions(downloads DESC);

-- ============================================================================
-- Full-text search optimization
-- ============================================================================

-- Ensure search vectors are updated (they should be auto-generated, but verify)
-- Note: These indexes already exist from previous migrations, but we ensure they're optimal

-- Analyze tables to update statistics for query planner
ANALYZE plugins;
ANALYZE templates;
ANALYZE scenarios;
ANALYZE plugin_versions;
ANALYZE template_versions;
ANALYZE scenario_versions;
ANALYZE reviews;
ANALYZE template_reviews;
ANALYZE scenario_reviews;
