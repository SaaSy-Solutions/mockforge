-- MockOps Coverage Metrics Migration
-- Migration 002: Coverage metrics for scenario usage, persona CI hits, endpoint coverage

-- ============================================================================
-- 1. Scenario Usage Metrics
-- ============================================================================
CREATE TABLE IF NOT EXISTS scenario_usage_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scenario_id TEXT NOT NULL,
    workspace_id TEXT,
    org_id TEXT,
    usage_count INTEGER NOT NULL DEFAULT 0,
    last_used_at INTEGER,
    usage_pattern TEXT,  -- JSON: Time-based usage patterns
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_scenario_usage_scenario ON scenario_usage_metrics(scenario_id);
CREATE INDEX IF NOT EXISTS idx_scenario_usage_workspace ON scenario_usage_metrics(workspace_id);
CREATE INDEX IF NOT EXISTS idx_scenario_usage_org ON scenario_usage_metrics(org_id);
CREATE INDEX IF NOT EXISTS idx_scenario_usage_last_used ON scenario_usage_metrics(last_used_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_scenario_usage_unique ON scenario_usage_metrics(scenario_id, COALESCE(workspace_id, ''), COALESCE(org_id, ''));

-- ============================================================================
-- 2. Persona CI Hits
-- ============================================================================
CREATE TABLE IF NOT EXISTS persona_ci_hits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    persona_id TEXT NOT NULL,
    workspace_id TEXT,
    org_id TEXT,
    ci_run_id TEXT,
    hit_count INTEGER NOT NULL DEFAULT 0,
    hit_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_persona_ci_persona ON persona_ci_hits(persona_id);
CREATE INDEX IF NOT EXISTS idx_persona_ci_workspace ON persona_ci_hits(workspace_id);
CREATE INDEX IF NOT EXISTS idx_persona_ci_org ON persona_ci_hits(org_id);
CREATE INDEX IF NOT EXISTS idx_persona_ci_run ON persona_ci_hits(ci_run_id);
CREATE INDEX IF NOT EXISTS idx_persona_ci_hit_at ON persona_ci_hits(hit_at DESC);

-- ============================================================================
-- 3. Endpoint Coverage
-- ============================================================================
CREATE TABLE IF NOT EXISTS endpoint_coverage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    endpoint TEXT NOT NULL,
    method TEXT,
    protocol TEXT NOT NULL,
    workspace_id TEXT,
    org_id TEXT,
    test_count INTEGER NOT NULL DEFAULT 0,
    last_tested_at INTEGER,
    coverage_percentage REAL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_endpoint_coverage_endpoint ON endpoint_coverage(endpoint);
CREATE INDEX IF NOT EXISTS idx_endpoint_coverage_workspace ON endpoint_coverage(workspace_id);
CREATE INDEX IF NOT EXISTS idx_endpoint_coverage_org ON endpoint_coverage(org_id);
CREATE INDEX IF NOT EXISTS idx_endpoint_coverage_protocol ON endpoint_coverage(protocol);
CREATE INDEX IF NOT EXISTS idx_endpoint_coverage_percentage ON endpoint_coverage(coverage_percentage);
CREATE UNIQUE INDEX IF NOT EXISTS idx_endpoint_coverage_unique ON endpoint_coverage(endpoint, COALESCE(method, ''), protocol, COALESCE(workspace_id, ''), COALESCE(org_id, ''));

-- ============================================================================
-- 4. Reality Level Staleness
-- ============================================================================
CREATE TABLE IF NOT EXISTS reality_level_staleness (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL,
    org_id TEXT,
    endpoint TEXT,
    method TEXT,
    protocol TEXT,
    current_reality_level TEXT,
    last_updated_at INTEGER,
    staleness_days INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_reality_staleness_workspace ON reality_level_staleness(workspace_id);
CREATE INDEX IF NOT EXISTS idx_reality_staleness_org ON reality_level_staleness(org_id);
CREATE INDEX IF NOT EXISTS idx_reality_staleness_endpoint ON reality_level_staleness(endpoint);
CREATE INDEX IF NOT EXISTS idx_reality_staleness_days ON reality_level_staleness(staleness_days DESC);
CREATE INDEX IF NOT EXISTS idx_reality_staleness_updated ON reality_level_staleness(last_updated_at);

-- ============================================================================
-- 5. Drift Percentage Metrics
-- ============================================================================
CREATE TABLE IF NOT EXISTS drift_percentage_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL,
    org_id TEXT,
    total_mocks INTEGER NOT NULL DEFAULT 0,
    drifting_mocks INTEGER NOT NULL DEFAULT 0,
    drift_percentage REAL NOT NULL,
    measured_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_drift_metrics_workspace ON drift_percentage_metrics(workspace_id);
CREATE INDEX IF NOT EXISTS idx_drift_metrics_org ON drift_percentage_metrics(org_id);
CREATE INDEX IF NOT EXISTS idx_drift_metrics_measured ON drift_percentage_metrics(measured_at DESC);
CREATE INDEX IF NOT EXISTS idx_drift_metrics_percentage ON drift_percentage_metrics(drift_percentage DESC);
