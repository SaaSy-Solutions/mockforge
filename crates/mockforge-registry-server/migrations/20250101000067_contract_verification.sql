-- Contract Diff / Verification / Fitness Functions
-- (cloud-enablement task #8 / Phase 1).
--
-- Persistent state for the managed drift-detection product. Probe runs
-- reuse the #4 worker pool with kind values 'contract_diff' /
-- 'verification_suite' / 'fitness_evaluation'. Drift findings raise
-- incidents through the #3 IncidentBus once integrated.
--
-- See docs/cloud/CLOUD_CONTRACT_VERIFICATION_DESIGN.md.

CREATE TABLE IF NOT EXISTS monitored_services (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    openapi_spec_url TEXT,
    openapi_spec_inline JSONB,                  -- alternative if URL unreachable
    auth_config JSONB,                          -- encrypted token / header via settings::encrypt
    traffic_source TEXT NOT NULL,               -- 'logs' | 'capture_session' | 'probe'
    traffic_source_ref TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_monitored_services_workspace
    ON monitored_services(workspace_id, enabled);

CREATE TABLE IF NOT EXISTS contract_diff_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    monitored_service_id UUID NOT NULL REFERENCES monitored_services(id) ON DELETE CASCADE,
    triggered_by TEXT NOT NULL,                 -- 'manual' | 'schedule'
    status TEXT NOT NULL,                       -- 'queued' | 'running' | 'passed' | 'failed' | 'errored'
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ,
    breaking_changes_count INTEGER NOT NULL DEFAULT 0,
    non_breaking_changes_count INTEGER NOT NULL DEFAULT 0,
    summary JSONB
);
CREATE INDEX IF NOT EXISTS idx_contract_diff_runs_service
    ON contract_diff_runs(monitored_service_id, started_at DESC);

CREATE TABLE IF NOT EXISTS contract_diff_findings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES contract_diff_runs(id) ON DELETE CASCADE,
    severity TEXT NOT NULL,                     -- 'breaking' | 'non_breaking' | 'cosmetic'
    endpoint TEXT NOT NULL,
    method TEXT,
    field_path TEXT,
    description TEXT NOT NULL,
    confidence DOUBLE PRECISION,                -- from ai_contract_diff::confidence_scorer
    suggested_fix JSONB                         -- from ai_contract_diff::correction_proposer
);
CREATE INDEX IF NOT EXISTS idx_contract_diff_findings_run
    ON contract_diff_findings(run_id, severity);

CREATE TABLE IF NOT EXISTS fitness_functions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- 'latency_threshold' | 'error_rate' | 'contract_stability' | 'custom_query'
    kind TEXT NOT NULL,
    config JSONB NOT NULL,                      -- threshold values, metric refs, time window
    last_evaluated_at TIMESTAMPTZ,
    last_status TEXT,                           -- 'pass' | 'fail' | 'unknown'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_fitness_functions_workspace
    ON fitness_functions(workspace_id, kind);

CREATE TABLE IF NOT EXISTS fitness_evaluations (
    id BIGSERIAL PRIMARY KEY,
    function_id UUID NOT NULL REFERENCES fitness_functions(id) ON DELETE CASCADE,
    status TEXT NOT NULL,                       -- 'pass' | 'fail' | 'unknown'
    measured_value DOUBLE PRECISION,
    threshold_value DOUBLE PRECISION,
    evaluated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_fitness_evaluations_function_time
    ON fitness_evaluations(function_id, evaluated_at DESC);

CREATE TABLE IF NOT EXISTS verification_suites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    contract_check_ids UUID[] NOT NULL DEFAULT '{}',
    fitness_function_ids UUID[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_verification_suites_workspace
    ON verification_suites(workspace_id);
