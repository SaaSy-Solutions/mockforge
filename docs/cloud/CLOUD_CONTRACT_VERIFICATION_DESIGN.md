# Cloud Contract Diff / Verification / Fitness Functions — Design

Cloud-enablement plan for `contract-diff`, `verification`, `fitness-functions` nav items. Tracks task #8 in the cloud-enablement plan.

## Goal

Move contract drift detection, schema verification, and fitness functions to a managed continuous-monitoring product. Customers register their deployed services + contract specs; the cloud runs scheduled checks and raises drift incidents when reality diverges from spec. Pricing dial: number of monitored services + check frequency.

This is the "enterprise gold" task — drift detection that watches deployed services and alerts is a known pain point for platform teams.

## What exists

- **`mockforge-core::ai_contract_diff`**: full diff pipeline with `diff_analyzer`, `semantic_analyzer`, `confidence_scorer`, `correction_proposer`, `recommendation_engine`.
- **UI pages**: `ContractDiffPage`, `VerificationPage`, `FitnessFunctionsPage` (all local-only).
- **Drift incidents**: today computed in-memory in the local UI. The cloud Incidents work (#3) gives us a persisted target for these.
- **Registry routes**: only email verification (unrelated). Zero contract/verification/fitness routes today.

## What's missing

1. **Monitored services registry.** Customers need to register `(name, base_url, openapi_spec_url, auth_config)` for what to monitor. New table.
2. **Scheduled contract checks.** Periodically:
   - Fetch live spec from `openapi_spec_url`.
   - Fetch sample traffic from registered traffic source (logs, capture session, or live probe endpoints).
   - Run the existing `ai_contract_diff` pipeline.
   - If breaking changes detected → raise incident via `IncidentBus` (#3).
3. **Fitness functions** as named, reusable assertions: "p99 latency under 200ms," "error rate under 1%," "no breaking schema changes per week." Each evaluates against time-series metrics or contract diffs.
4. **Verification suites**: a saved, runnable bundle of (contract checks + fitness functions) that can be invoked manually or scheduled.
5. **Trend tracking**: drift over time, not just point-in-time. Customers want "show me what changed in the last 30 days."
6. **Probe runner**: a worker that hits `base_url` to fetch live behavior for diff. Reuse the #4 worker pool.

## Cloud architecture

```
[ User registers service ]
            │
            ▼
   monitored_services row
            │
            ▼
   [ Scheduler triggers check every N hours ]
            │
            ▼
   [ Worker (reusing #4 pool) fetches spec + samples ]
            │
            ▼
   [ ai_contract_diff pipeline runs ]
            │
            ▼
   contract_diff_runs row + per-mismatch findings
            │
   ┌────────┴────────┐
   │                 │
   ▼                 ▼
Breaking?     Within fitness?
   │                 │
   ▼                 ▼
IncidentBus.raise()  pass/fail logged
```

### Proposed routes

```
# Service registry
GET    /api/v1/workspaces/{workspace_id}/monitored-services
POST   /api/v1/workspaces/{workspace_id}/monitored-services
GET    /api/v1/monitored-services/{id}
PATCH  /api/v1/monitored-services/{id}
DELETE /api/v1/monitored-services/{id}

# Contract diff
POST   /api/v1/monitored-services/{id}/diff                    # run check now
GET    /api/v1/monitored-services/{id}/diffs                   # history
GET    /api/v1/contract-diff-runs/{id}
GET    /api/v1/contract-diff-runs/{id}/findings

# Fitness functions
GET    /api/v1/workspaces/{workspace_id}/fitness-functions
POST   /api/v1/workspaces/{workspace_id}/fitness-functions
PATCH  /api/v1/fitness-functions/{id}
DELETE /api/v1/fitness-functions/{id}
POST   /api/v1/fitness-functions/{id}/evaluate                 # run now

# Verification suites
GET    /api/v1/workspaces/{workspace_id}/verification-suites
POST   /api/v1/workspaces/{workspace_id}/verification-suites
POST   /api/v1/verification-suites/{id}/run                    # uses test_runs lifecycle from #4

# Schedules
POST   /api/v1/monitored-services/{id}/schedules
POST   /api/v1/verification-suites/{id}/schedules
POST   /api/v1/fitness-functions/{id}/schedules
```

## Data model

```sql
CREATE TABLE monitored_services (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    openapi_spec_url TEXT,
    openapi_spec_inline JSONB,                  -- alternative if URL unreachable
    auth_config JSONB,                          -- encrypted token / header
    traffic_source TEXT NOT NULL,               -- 'logs' | 'capture_session' | 'probe'
    traffic_source_ref TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE contract_diff_runs (
    id UUID PRIMARY KEY,
    monitored_service_id UUID NOT NULL REFERENCES monitored_services(id) ON DELETE CASCADE,
    triggered_by TEXT NOT NULL,                 -- 'manual' | 'schedule'
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ,
    breaking_changes_count INT NOT NULL DEFAULT 0,
    non_breaking_changes_count INT NOT NULL DEFAULT 0,
    summary JSONB
);

CREATE TABLE contract_diff_findings (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES contract_diff_runs(id) ON DELETE CASCADE,
    severity TEXT NOT NULL,                     -- 'breaking' | 'non_breaking' | 'cosmetic'
    endpoint TEXT NOT NULL,
    method TEXT,
    field_path TEXT,
    description TEXT NOT NULL,
    confidence DOUBLE PRECISION,                -- from confidence_scorer.rs
    suggested_fix JSONB                         -- from correction_proposer.rs
);

CREATE TABLE fitness_functions (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    kind TEXT NOT NULL,                         -- 'latency_threshold' | 'error_rate' | 'contract_stability' | 'custom_query'
    config JSONB NOT NULL,                      -- threshold values, metric refs, time window
    last_evaluated_at TIMESTAMPTZ,
    last_status TEXT,                           -- 'pass' | 'fail' | 'unknown'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE fitness_evaluations (
    id BIGSERIAL PRIMARY KEY,
    function_id UUID NOT NULL REFERENCES fitness_functions(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    measured_value DOUBLE PRECISION,
    threshold_value DOUBLE PRECISION,
    evaluated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE verification_suites (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    contract_check_ids UUID[],                  -- monitored_services
    fitness_function_ids UUID[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Plan tiers

- **Free**: 1 monitored service, daily checks only, no fitness functions.
- **Pro**: 5 services, hourly checks, 10 fitness functions.
- **Team**: 25 services, 15-minute checks, 50 fitness functions, scheduled verification suites.
- **Enterprise**: unlimited services, 1-minute checks, custom check intervals, AI-powered correction proposer access.

Worker time still meters against `runner_seconds_used` (#4) — frequent checks burn through it.

## Integration with other tasks

- **#3 Incidents**: Breaking-change findings call `IncidentBus.raise()` with `source = 'drift'`, `dedupe_key = endpoint:method`. Resolves automatically when next clean check passes.
- **#2 Observability**: Fitness functions of kind `latency_threshold` / `error_rate` query the metrics tables.
- **#1 AI Studio**: `correction_proposer.rs` and `recommendation_engine.rs` call the cloud LLM proxy for AI-driven suggestions; counts against the org's AI tokens.
- **#4 Test Execution**: Verification suite runs use the `test_runs` lifecycle.

## UI changes

1. `AppShell.tsx:217` — add `'contract-diff'`, `'verification'`, `'fitness-functions'` to `cloudNavItemIds`.
2. **ContractDiffPage rewrite**: monitored-service registry, schedule editor, diff history, finding-detail drawer with confidence-scored fixes.
3. **FitnessFunctionsPage rewrite**: function list, create/edit form per kind, evaluation history sparkline, last-status badge.
4. **VerificationPage rewrite**: suite list, suite editor (multi-select services + functions), schedule, run history.
5. **Cross-page deep links**: a finding links to "Run a verification suite around this," etc.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Schema migrations | ~1.5 days |
| 2 | monitored_services CRUD + auth_config encryption | ~1.5 days |
| 3 | Probe worker (fetches spec + traffic, runs ai_contract_diff) | ~2 days |
| 4 | Diff results storage + IncidentBus wiring | ~1.5 days |
| 5 | Fitness function evaluator (per kind) | ~2 days |
| 6 | Verification suite composition + run trigger | ~1 day |
| 7 | Schedule worker (cron evaluator across all 3 schedule kinds) | ~1.5 days |
| 8 | UI rewrites | ~3 days |
| 9 | E2E (register service → schedule check → drift detected → incident raised) | ~1.5 days |

Total: ~15 working days for v1 (assumes #1, #3, #4 done first).

## Decisions

### Reuse #4 worker pool

**Decision: yes.** Probe runs and verification-suite runs are wall-clock work that fits on-demand Fly machines. New `kind` values (`contract_diff`, `verification_suite`, `fitness_evaluation`).

### Confidence scoring exposed in UI

**Decision: yes.** `ai_contract_diff::confidence_scorer` already produces per-finding confidence. Surface it in the UI (e.g., "85% confidence this is breaking") so users can prioritize. Helps with the noise problem on diff tools.

### Auto-resolve drift incidents

**Decision: yes — when next clean check passes for the same dedupe_key.** Manual-only resolve creates noise; clean-check auto-resolve keeps the incident list meaningful.

## Out of scope for v1

- Multi-version contract tracking (we always compare current-spec vs. live; no per-version drift).
- Custom assertion DSL beyond the four `kind` types.
- Cross-service drift (e.g., service A's responses break service B's expectations).
- Drift forecasting / regression risk scoring.

## Open questions

1. The local UI's `driftApi` shape — does it survive cloud-mode, or does the UI fully switch to the new shape? Recommend full switch; fewer code paths.
2. Probe traffic against a customer's prod has the same blast-radius concern as #7 chaos. Smaller though — we're just sending sample requests. Probably need a 1-RPS cap on probes by default.
3. Fitness function `custom_query` kind would let users write SQL/PromQL — flexible but a security risk and schema-coupling nightmare. Defer to v2 or add a sandboxed expression language.
