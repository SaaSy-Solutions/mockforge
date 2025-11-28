-- Pillar Usage Tracking schema
-- Tracks usage of MockForge pillars (Reality, Contracts, DevX, Cloud, AI)
-- to help users understand platform adoption and identify under-utilized features

-- Pillar usage events table
CREATE TABLE IF NOT EXISTS pillar_usage_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT,
    org_id TEXT,
    pillar TEXT NOT NULL CHECK (pillar IN ('reality', 'contracts', 'devx', 'cloud', 'ai')),
    metric_name TEXT NOT NULL,
    metric_value TEXT NOT NULL, -- JSON string for flexibility
    timestamp INTEGER NOT NULL, -- Unix timestamp
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_pillar_usage_workspace ON pillar_usage_events(workspace_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_pillar_usage_org ON pillar_usage_events(org_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_pillar_usage_pillar ON pillar_usage_events(pillar, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_pillar_usage_metric ON pillar_usage_events(metric_name, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_pillar_usage_timestamp ON pillar_usage_events(timestamp DESC);

-- Composite indexes for common queries
CREATE INDEX IF NOT EXISTS idx_pillar_usage_workspace_pillar ON pillar_usage_events(workspace_id, pillar, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_pillar_usage_org_pillar ON pillar_usage_events(org_id, pillar, timestamp DESC);

-- Add comments for documentation
-- Note: SQLite doesn't support comments on tables/columns, but we document here
-- pillar_usage_events: Tracks pillar usage events for analytics
-- pillar: One of 'reality', 'contracts', 'devx', 'cloud', 'ai'
-- metric_name: Name of the metric (e.g., 'blended_reality_ratio', 'smart_personas_usage', 'validation_mode')
-- metric_value: JSON string containing the metric value and metadata
