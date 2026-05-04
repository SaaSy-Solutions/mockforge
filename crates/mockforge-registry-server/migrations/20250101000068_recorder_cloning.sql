-- Recorder + Behavioral Cloning extensions (cloud-enablement task #6 / Phase 1).
--
-- The runtime_captures table already exists (migration 20250101000053);
-- this slice adds:
-- - workspace_id + source columns for org-scoped queries
-- - capture_sessions + capture_session_members for grouping captures
-- - clone_models for behavioral-cloning training output
-- - usage_counters.capture_bytes_stored gauge
--
-- Training jobs reuse the #4 worker pool with kind='behavioral_clone';
-- replay reuses test_runs with kind='replay'. See
-- docs/cloud/CLOUD_RECORDER_BEHAVIORAL_CLONING_DESIGN.md.

-- 1. Org-scoping the existing captures table.
ALTER TABLE runtime_captures
    ADD COLUMN IF NOT EXISTS workspace_id UUID;
ALTER TABLE runtime_captures
    ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'hosted';
-- 'hosted' = shipped from a hosted-mock container (existing flow);
-- 'local'  = shipped from a locally-running mockforge via --cloud-ship.

CREATE INDEX IF NOT EXISTS idx_runtime_captures_workspace
    ON runtime_captures(workspace_id)
    WHERE workspace_id IS NOT NULL;

-- 2. Capture sessions — named groupings users build for training.
CREATE TABLE IF NOT EXISTS capture_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    capture_count INTEGER NOT NULL DEFAULT 0,
    total_bytes BIGINT NOT NULL DEFAULT 0,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_capture_sessions_workspace
    ON capture_sessions(workspace_id, updated_at DESC);

-- We can't FK capture_id → runtime_captures.id because runtime_captures
-- already exists in production with its own row id type — leave the
-- FK off and rely on CASCADE-style cleanup at the application layer
-- if/when captures are deleted.
CREATE TABLE IF NOT EXISTS capture_session_members (
    session_id UUID NOT NULL REFERENCES capture_sessions(id) ON DELETE CASCADE,
    capture_id UUID NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (session_id, capture_id)
);
CREATE INDEX IF NOT EXISTS idx_capture_session_members_capture
    ON capture_session_members(capture_id);

-- 3. Trained behavioral-clone models.
CREATE TABLE IF NOT EXISTS clone_models (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    source_session_id UUID REFERENCES capture_sessions(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'training',    -- 'training' | 'ready' | 'failed'
    artifact_url TEXT,                          -- blob storage URL for the model
    metrics JSONB,                              -- accuracy, coverage, latency P50/P99
    runner_seconds INTEGER,
    deployed_to UUID,                           -- hosted_deployments.id (no FK, OSS-friendly)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_clone_models_workspace
    ON clone_models(workspace_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_clone_models_status
    ON clone_models(status, created_at);

-- 4. Storage gauge for the dashboard quota meter (gauge, not counter —
-- like snapshot_bytes_stored).
ALTER TABLE usage_counters
    ADD COLUMN IF NOT EXISTS capture_bytes_stored BIGINT NOT NULL DEFAULT 0;
