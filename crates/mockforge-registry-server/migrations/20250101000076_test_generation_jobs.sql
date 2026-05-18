-- Cloud Test Generator — async LLM jobs over the workspace's runtime_captures
-- corpus (#469).
--
-- Each row is one async generation request: the org's BYOK provider key
-- backs the LLM call (rate-limited via the existing ai::quota path) and
-- the resulting test scenarios land in the `result` column once status
-- transitions out of 'queued'/'running'.
--
-- Phase 1 (this migration) ships the data plane only — table + indexes.
-- Phase 2 will wire the background worker that pulls queued rows, calls
-- the BYOK LLM, and persists results.

CREATE TABLE cloud_test_generation_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    -- 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled'
    status TEXT NOT NULL DEFAULT 'queued',
    -- Free-form natural-language prompt describing what tests to generate.
    -- Empty string allowed — the worker falls back to a default prompt
    -- derived from the captures_filter.
    prompt TEXT NOT NULL DEFAULT '',
    -- Filter applied when selecting captures from runtime_captures. JSONB
    -- so we can evolve the filter vocabulary (path globs, status code
    -- ranges, time window, sample size, etc.) without a schema migration.
    captures_filter JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- LLM-generated test scenarios. NULL until the worker finishes a
    -- 'succeeded' run; the shape is `{"scenarios": [...], "model": "...",
    -- "captures_sampled": N}` per the upstream Test Generator format.
    result JSONB,
    -- Human-readable failure reason. NULL unless status = 'failed' or
    -- 'cancelled'. Populated by the worker on failure paths.
    error TEXT,
    -- Bookkeeping for billing + UI progress. queued_at = row creation;
    -- started_at = worker picked it up; finished_at = terminal status
    -- ('succeeded' | 'failed' | 'cancelled').
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL
);

-- Listing the workspace's recent jobs, newest first — the cloud
-- TestGeneratorPage's primary view.
CREATE INDEX idx_cloud_test_gen_jobs_workspace
    ON cloud_test_generation_jobs(workspace_id, queued_at DESC);

-- Worker scan: `WHERE status IN ('queued','running')` is the queue feed.
-- A partial index keeps the index small even as the finished-jobs table
-- grows unbounded.
CREATE INDEX idx_cloud_test_gen_jobs_pending
    ON cloud_test_generation_jobs(queued_at)
    WHERE status IN ('queued', 'running');

-- Org-wide views (admin / billing). Lets us count active runs per org
-- without seq-scanning every workspace.
CREATE INDEX idx_cloud_test_gen_jobs_org_status
    ON cloud_test_generation_jobs(org_id, status);
