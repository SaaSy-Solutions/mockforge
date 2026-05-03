-- Test Execution suite (cloud-enablement task #4 / Phase 1).
--
-- Persists user-authored test suites, queues + tracks runs, streams events
-- from cloud workers, schedules cron-driven runs, and attaches run artifacts.
-- Workers (mockforge-test-runner — separate crate, future PR) consume jobs
-- via a Redis queue; the registry only stores state and serves the UI.
--
-- Reused by other cloud-enablement tasks via the `kind` column:
-- - kind='unit'|'integration'|'conformance'|'bench'|'owasp' from #4
-- - kind='chaos_campaign' from #7
-- - kind='behavioral_clone' from #6
-- - kind='contract_diff'|'verification_suite'|'fitness_evaluation' from #8
-- - kind='scenario'|'orchestration'|'state_machine'|'chain' from #9
-- - kind='snapshot_capture'|'snapshot_restore' from #10
-- - kind='replay' from #6
--
-- Keeping `kind` open (TEXT, not enum) avoids needing a schema migration
-- every time we add a new flow type.

CREATE TABLE IF NOT EXISTS test_suites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    kind TEXT NOT NULL,                         -- see header for vocabulary
    config JSONB NOT NULL,                      -- kind-specific definition
    target_workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_test_suites_workspace
    ON test_suites(workspace_id, kind);

CREATE TABLE IF NOT EXISTS test_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    suite_id UUID NOT NULL REFERENCES test_suites(id) ON DELETE CASCADE,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    triggered_by TEXT NOT NULL,                 -- 'manual' | 'schedule' | 'ci' | 'webhook'
    triggered_by_user UUID REFERENCES users(id) ON DELETE SET NULL,
    status TEXT NOT NULL,                       -- 'queued' | 'running' | 'passed' | 'failed' | 'cancelled' | 'errored'
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    runner_seconds INTEGER,                     -- wall-clock; bills against usage_counters.runner_seconds_used
    summary JSONB,                              -- pass/fail counts, p50/p99, etc.
    git_ref TEXT,                               -- when triggered from CI
    git_sha TEXT
);
CREATE INDEX IF NOT EXISTS idx_test_runs_suite_finished
    ON test_runs(suite_id, finished_at DESC);
CREATE INDEX IF NOT EXISTS idx_test_runs_org_status
    ON test_runs(org_id, status);
-- Supports `WHERE status IN ('queued','running')` queue scans without
-- touching finished rows.
CREATE INDEX IF NOT EXISTS idx_test_runs_inflight
    ON test_runs(org_id, queued_at)
    WHERE status IN ('queued', 'running');

CREATE TABLE IF NOT EXISTS test_run_events (
    id BIGSERIAL PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES test_runs(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,                       -- ordering within a run
    event_type TEXT NOT NULL,                   -- 'step_start' | 'step_pass' | 'step_fail' | 'log' | 'metric'
    payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (run_id, seq)
);
-- The (run_id, seq) UNIQUE doubles as the streaming-replay index.

CREATE TABLE IF NOT EXISTS test_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    suite_id UUID NOT NULL REFERENCES test_suites(id) ON DELETE CASCADE,
    cron TEXT NOT NULL,                         -- "0 2 * * *"
    timezone TEXT NOT NULL DEFAULT 'UTC',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_triggered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_test_schedules_suite
    ON test_schedules(suite_id);
CREATE INDEX IF NOT EXISTS idx_test_schedules_enabled
    ON test_schedules(enabled, last_triggered_at)
    WHERE enabled = TRUE;

CREATE TABLE IF NOT EXISTS test_run_artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES test_runs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    storage_url TEXT NOT NULL,                  -- blob storage (Fly volumes / S3)
    size_bytes BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_test_run_artifacts_run
    ON test_run_artifacts(run_id);

-- New billing meter for runner time. Mirrors ai_tokens_used. Plan limits
-- live in organizations.limits_json under "runner_seconds_per_month".
ALTER TABLE usage_counters
    ADD COLUMN IF NOT EXISTS runner_seconds_used BIGINT NOT NULL DEFAULT 0;
