-- Time Travel snapshots (cloud-enablement task #10 / Phase 1).
--
-- Persistent snapshot rows + schedules. Actual capture/restore work
-- runs on the #4 Test Execution worker pool with new test_runs.kind
-- values (`snapshot_capture` / `snapshot_restore`); this schema just
-- holds the metadata + blob-storage references.
--
-- See docs/cloud/CLOUD_TIME_TRAVEL_DESIGN.md.

CREATE TABLE IF NOT EXISTS snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    hosted_deployment_id UUID,                  -- optional FK; can't reference here
                                                -- because hosted_deployments may not
                                                -- exist in OSS deployments
    name TEXT,
    description TEXT,
    triggered_by TEXT NOT NULL,                 -- 'manual' | 'schedule' | 'pre_chaos' | 'pre_restore'
    triggered_by_user UUID REFERENCES users(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'capturing',   -- 'capturing' | 'ready' | 'failed' | 'expired'
    storage_url TEXT,                           -- blob storage; set on transition to 'ready'
    size_bytes BIGINT,                          -- ditto
    manifest JSONB,                             -- "what's included" component summary
    expires_at TIMESTAMPTZ,                     -- driven by plan retention; NULL = never
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    captured_at TIMESTAMPTZ                     -- set when status becomes 'ready'
);
-- Recent-snapshots view per workspace.
CREATE INDEX IF NOT EXISTS idx_snapshots_workspace_created
    ON snapshots(workspace_id, created_at DESC);
-- Retention worker scan: find ready snapshots past their expiry.
CREATE INDEX IF NOT EXISTS idx_snapshots_expires_ready
    ON snapshots(expires_at)
    WHERE status = 'ready' AND expires_at IS NOT NULL;
-- Hosted-deployment-scoped view (e.g., "snapshots of this mock").
CREATE INDEX IF NOT EXISTS idx_snapshots_deployment
    ON snapshots(hosted_deployment_id, created_at DESC)
    WHERE hosted_deployment_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS snapshot_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    cron TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_triggered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_snapshot_schedules_workspace
    ON snapshot_schedules(workspace_id);
CREATE INDEX IF NOT EXISTS idx_snapshot_schedules_enabled
    ON snapshot_schedules(enabled, last_triggered_at)
    WHERE enabled = TRUE;

-- Storage quota meter. Mirrors the other usage_counters columns.
ALTER TABLE usage_counters
    ADD COLUMN IF NOT EXISTS snapshot_bytes_stored BIGINT NOT NULL DEFAULT 0;
