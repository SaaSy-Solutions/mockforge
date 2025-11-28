-- Performance optimization: Add missing indexes for common query patterns
-- This migration adds indexes to improve query performance for frequently accessed data

-- ============================================================================
-- Users table indexes
-- ============================================================================

-- Index for email lookups (login, user lookup)
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Index for username lookups (already has UNIQUE constraint, but explicit index helps)
-- Note: UNIQUE constraint already creates an index, but we'll keep this for clarity

-- Index for verification status (filtering unverified users)
CREATE INDEX IF NOT EXISTS idx_users_verified ON users(is_verified) WHERE is_verified = FALSE;

-- ============================================================================
-- API tokens table indexes
-- ============================================================================

-- Index for token rotation detection (queries by created_at)
CREATE INDEX IF NOT EXISTS idx_api_tokens_created ON api_tokens(created_at);

-- Index for expired token filtering (queries filtering by expires_at)
CREATE INDEX IF NOT EXISTS idx_api_tokens_expires ON api_tokens(expires_at) WHERE expires_at IS NOT NULL;

-- Composite index for org + created_at (common pattern for listing tokens)
CREATE INDEX IF NOT EXISTS idx_api_tokens_org_created ON api_tokens(org_id, created_at DESC);

-- Index for last_used_at (tracking token usage)
CREATE INDEX IF NOT EXISTS idx_api_tokens_last_used ON api_tokens(last_used_at) WHERE last_used_at IS NOT NULL;

-- ============================================================================
-- Subscriptions table indexes
-- ============================================================================

-- Composite index for org + status (common query pattern)
CREATE INDEX IF NOT EXISTS idx_subscriptions_org_status ON subscriptions(org_id, status);

-- Index for period queries (finding active subscriptions in date range)
CREATE INDEX IF NOT EXISTS idx_subscriptions_period ON subscriptions(current_period_start, current_period_end);

-- ============================================================================
-- Templates table indexes
-- ============================================================================

-- Composite index for org + published (common search pattern)
CREATE INDEX IF NOT EXISTS idx_templates_org_published ON templates(org_id, published) WHERE published = TRUE;

-- Composite index for org + category (filtering by category within org)
CREATE INDEX IF NOT EXISTS idx_templates_org_category ON templates(org_id, category);

-- Index for created_at sorting (recent templates)
CREATE INDEX IF NOT EXISTS idx_templates_created ON templates(created_at DESC);

-- Index for tags array searches (GIN index for array containment)
CREATE INDEX IF NOT EXISTS idx_templates_tags ON templates USING GIN(tags);

-- Composite index for name + version (unique constraint lookup)
-- Note: UNIQUE constraint already creates an index, but composite helps queries

-- ============================================================================
-- Scenarios table indexes
-- ============================================================================

-- Composite index for org + category (filtering by category within org)
CREATE INDEX IF NOT EXISTS idx_scenarios_org_category ON scenarios(org_id, category);

-- Index for created_at sorting (recent scenarios)
CREATE INDEX IF NOT EXISTS idx_scenarios_created ON scenarios(created_at DESC);

-- Index for tags array searches (GIN index for array containment)
CREATE INDEX IF NOT EXISTS idx_scenarios_tags ON scenarios USING GIN(tags);

-- Index for downloads sorting (popular scenarios)
CREATE INDEX IF NOT EXISTS idx_scenarios_downloads ON scenarios(downloads_total DESC);

-- Index for rating sorting (highly rated scenarios)
CREATE INDEX IF NOT EXISTS idx_scenarios_rating ON scenarios(rating_avg DESC) WHERE rating_count > 0;

-- ============================================================================
-- Plugins table indexes
-- ============================================================================

-- Composite index for org + name (common lookup pattern)
CREATE INDEX IF NOT EXISTS idx_plugins_org_name ON plugins(org_id, name) WHERE org_id IS NOT NULL;

-- Composite index for org + category (filtering by category within org)
CREATE INDEX IF NOT EXISTS idx_plugins_org_category ON plugins(org_id, category) WHERE org_id IS NOT NULL;

-- Index for created_at sorting (recent plugins)
CREATE INDEX IF NOT EXISTS idx_plugins_created ON plugins(created_at DESC);

-- ============================================================================
-- Settings tables indexes
-- ============================================================================

-- Composite index for org settings lookups (org_id + setting_key)
CREATE INDEX IF NOT EXISTS idx_org_settings_lookup ON org_settings(org_id, setting_key);

-- Composite index for user settings lookups (user_id + setting_key)
CREATE INDEX IF NOT EXISTS idx_user_settings_lookup ON user_settings(user_id, setting_key);

-- ============================================================================
-- Hosted mocks table indexes
-- ============================================================================

-- Composite index for org + status (common query pattern)
CREATE INDEX IF NOT EXISTS idx_hosted_mocks_org_status ON hosted_mocks(org_id, status);

-- Composite index for org + slug (routing lookups)
CREATE INDEX IF NOT EXISTS idx_hosted_mocks_org_slug ON hosted_mocks(org_id, slug) WHERE deleted_at IS NULL;

-- Index for health status filtering
CREATE INDEX IF NOT EXISTS idx_hosted_mocks_health ON hosted_mocks(health_status) WHERE health_status != 'unknown';

-- ============================================================================
-- Deployment logs table indexes
-- ============================================================================

-- Composite index for mock + created_at (recent logs for a deployment)
CREATE INDEX IF NOT EXISTS idx_deployment_logs_mock_created ON deployment_logs(hosted_mock_id, created_at DESC);

-- Index for level filtering (error logs)
CREATE INDEX IF NOT EXISTS idx_deployment_logs_level ON deployment_logs(level) WHERE level IN ('error', 'warning');

-- ============================================================================
-- Review tables indexes
-- ============================================================================

-- Index for plugin reviews by rating
CREATE INDEX IF NOT EXISTS idx_reviews_plugin_rating ON reviews(plugin_id, rating);

-- Index for template reviews by rating
CREATE INDEX IF NOT EXISTS idx_template_reviews_rating ON template_reviews(template_id, rating);

-- Index for scenario reviews by rating
CREATE INDEX IF NOT EXISTS idx_scenario_reviews_rating ON scenario_reviews(scenario_id, rating);

-- ============================================================================
-- OAuth accounts table indexes (if exists)
-- ============================================================================

-- Index for user lookups via OAuth
-- Note: Check if oauth_accounts table exists first
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'oauth_accounts') THEN
        CREATE INDEX IF NOT EXISTS idx_oauth_accounts_user ON oauth_accounts(user_id);
        CREATE INDEX IF NOT EXISTS idx_oauth_accounts_provider ON oauth_accounts(provider, provider_user_id);
    END IF;
END $$;

-- ============================================================================
-- Verification tokens table indexes
-- ============================================================================

-- Composite index for user + expires_at (finding valid tokens)
CREATE INDEX IF NOT EXISTS idx_verification_tokens_user_expires ON verification_tokens(user_id, expires_at) WHERE used_at IS NULL;

-- ============================================================================
-- Usage counters table indexes
-- ============================================================================

-- Index for period queries (finding counters by date range)
CREATE INDEX IF NOT EXISTS idx_usage_counters_period_start ON usage_counters(period_start);

-- ============================================================================
-- Feature usage table indexes
-- ============================================================================

-- Composite index for org + feature + created_at (analytics queries)
CREATE INDEX IF NOT EXISTS idx_feature_usage_org_feature_created ON feature_usage(org_id, feature, created_at DESC);

-- ============================================================================
-- Audit logs table indexes
-- ============================================================================

-- Composite index for org + event_type + created_at (filtered audit queries)
CREATE INDEX IF NOT EXISTS idx_audit_logs_org_event_created ON audit_logs(org_id, event_type, created_at DESC);

-- ============================================================================
-- Plugin versions table indexes
-- ============================================================================

-- Index for yanked filtering (excluding yanked versions)
CREATE INDEX IF NOT EXISTS idx_plugin_versions_yanked ON plugin_versions(plugin_id, yanked) WHERE yanked = FALSE;

-- Index for downloads sorting
CREATE INDEX IF NOT EXISTS idx_plugin_versions_downloads ON plugin_versions(downloads DESC);

-- ============================================================================
-- Template/Scenario versions table indexes
-- ============================================================================

-- Index for yanked filtering
CREATE INDEX IF NOT EXISTS idx_template_versions_yanked ON template_versions(template_id, yanked) WHERE yanked = FALSE;

CREATE INDEX IF NOT EXISTS idx_scenario_versions_yanked ON scenario_versions(scenario_id, yanked) WHERE yanked = FALSE;

-- ============================================================================
-- Analysis and statistics
-- ============================================================================

-- Note: After applying this migration, you may want to run ANALYZE on tables
-- to update statistics for the query planner:
-- ANALYZE users;
-- ANALYZE api_tokens;
-- ANALYZE subscriptions;
-- ANALYZE templates;
-- ANALYZE scenarios;
-- ANALYZE plugins;
-- ANALYZE hosted_mocks;
-- ANALYZE feature_usage;
-- ANALYZE audit_logs;
