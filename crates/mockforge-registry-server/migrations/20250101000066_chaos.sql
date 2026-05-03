-- Chaos + Resilience (cloud-enablement task #7 / Phase 1).
--
-- Persistent state for managed chaos campaigns. Run execution reuses
-- the #4 Test Execution worker pool with kind='chaos_campaign'; the
-- chaos_campaign_reports.run_id FK ties a chaos report to the
-- generic test_runs row that worked it.
--
-- See docs/cloud/CLOUD_CHAOS_RESILIENCE_DESIGN.md.

CREATE TABLE IF NOT EXISTS chaos_campaigns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    target_kind TEXT NOT NULL,                  -- 'hosted_mock' | 'external'
    target_ref TEXT NOT NULL,                   -- deployment_id or URL
    config JSONB NOT NULL,                      -- fault types, intensities, schedule
    safety_config JSONB NOT NULL,               -- kill-switch thresholds
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_chaos_campaigns_workspace
    ON chaos_campaigns(workspace_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS chaos_campaign_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    campaign_id UUID NOT NULL REFERENCES chaos_campaigns(id) ON DELETE CASCADE,
    run_id UUID NOT NULL REFERENCES test_runs(id) ON DELETE CASCADE,
    fault_count INTEGER NOT NULL DEFAULT 0,
    aborted BOOLEAN NOT NULL DEFAULT FALSE,
    abort_reason TEXT,
    summary JSONB,                              -- p50/p99 before/during/after, error rates
    recommendations JSONB,                      -- from mockforge-chaos::recommendations
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_chaos_reports_campaign
    ON chaos_campaign_reports(campaign_id, created_at DESC);

CREATE TABLE IF NOT EXISTS resilience_patterns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- NULL workspace_id = platform-provided pattern available to everyone.
    workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,                         -- 'circuit_breaker' | 'retry' | 'bulkhead' | 'rate_limit'
    name TEXT NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_resilience_patterns_workspace
    ON resilience_patterns(workspace_id);
CREATE INDEX IF NOT EXISTS idx_resilience_patterns_platform
    ON resilience_patterns(kind)
    WHERE workspace_id IS NULL;
