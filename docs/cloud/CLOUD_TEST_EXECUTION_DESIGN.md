# Cloud Test Execution Suite — Design

Cloud-enablement plan for the test nav items (`testing`, `test-generator`, `test-execution`, `integration-test-builder`, `conformance`). Tracks task #4 in the cloud-enablement plan.

## Goal

Move test execution to cloud as a CI-grade managed runner: users define tests in the UI (or push from CI), runs execute on cloud workers, results persist with run history and diff against previous runs. Pricing dial: runner-minutes consumed, plus concurrency caps. Cypress Cloud / TestRail playbook.

## What exists locally

- **Pages**: `TestingPage`, `TestGeneratorPage`, `TestExecutionDashboard`, `IntegrationTestBuilder`, `ConformancePage`.
- **Backend crates**: `mockforge-test` (basic test framework), `mockforge-bench` (k6 generation, conformance, OWASP, chunked bench, parallel executor).
- **Local execution**: tests run in the same process or spawn k6 against a local mockforge.

## What's missing in cloud

There are **no test-related routes in the registry server today**. Everything below is greenfield.

1. **Test definitions persisted server-side.** Today tests live in workspace fixtures (locally edited). Cloud needs a first-class `test_suites` resource per workspace.
2. **Run orchestration.** A "run" is one execution of a suite. Needs queueing (Redis), worker pool, status updates (queued → running → passed/failed/cancelled), per-step results, log streaming.
3. **Cloud worker pool.** The actual test runner. Two implementation paths:
   - **(a) Long-lived worker fleet**: dedicated runner containers polling the queue.
   - **(b) On-demand**: spawn a Fly.io machine per run, terminate after.
   - Recommend **(b)** for v1: simpler ops, scales to zero, fits Fly's per-second billing. Cold-start cost (~3-5s) is acceptable for test runs that average minutes.
4. **Runner-minute metering.** New `usage_counters.runner_seconds_used` column, mirror of `ai_tokens_used` and `log_bytes_ingested`. Increment on run completion based on wall-clock.
5. **CI integration.** A `mockforge test run --suite=<id> --token=<api_token>` CLI command that triggers a cloud run from a pipeline, streams logs, and exits with the suite's pass/fail status.
6. **Result diff.** Compare current run to last green run; surface new failures vs. flaky vs. fixed.
7. **Schedule**. Cron-style schedules per suite (e.g., "run nightly at 02:00 UTC").

## Cloud architecture

```
[ User edits suite in UI ] ──▶ POST .../test-suites/{id}/runs
                                       │
                                       ▼
                           Redis queue: runner_jobs
                                       │
                                       ▼
                       [ Fly.io machine spawned per job ]
                                       │
                                       ▼
                     Runs k6/test/conformance against target
                                       │
                                       ├── Streams logs/events ──▶ Postgres + Redis pubsub
                                       │
                                       └── On exit: report final status, runner_seconds
                                       │
                                       ▼
                          Update test_runs.status, increment usage_counter
```

Workers are stateless; suite definition is downloaded at job start, results uploaded at exit.

### Proposed routes

```
GET    /api/v1/workspaces/{workspace_id}/test-suites
POST   /api/v1/workspaces/{workspace_id}/test-suites
GET    /api/v1/workspaces/{workspace_id}/test-suites/{id}
PATCH  /api/v1/workspaces/{workspace_id}/test-suites/{id}
DELETE /api/v1/workspaces/{workspace_id}/test-suites/{id}

POST   /api/v1/test-suites/{id}/runs                    # trigger run
GET    /api/v1/test-suites/{id}/runs                    # list run history
GET    /api/v1/test-runs/{run_id}
GET    /api/v1/test-runs/{run_id}/events/stream         # SSE for live tail
GET    /api/v1/test-runs/{run_id}/artifacts/{name}      # download HTML report, k6 summary, etc.
POST   /api/v1/test-runs/{run_id}/cancel

POST   /api/v1/workspaces/{workspace_id}/test-suites/{id}/schedules
DELETE /api/v1/test-suites/{id}/schedules/{schedule_id}

POST   /api/v1/test-suites/{id}/generate                # AI-driven test generation (delegates to ai-studio cloud)
```

### Suite kinds

`test_suites.kind` distinguishes:
- `unit` — `mockforge-test` style fixture-based tests.
- `integration` — `IntegrationTestBuilder` flows.
- `conformance` — OpenAPI conformance from `mockforge-bench::conformance`.
- `bench` — k6 load test (existing bench template).
- `owasp` — OWASP API security suite from `mockforge-bench::owasp_api`.

Each kind has its own runner config; all share the `test_runs` lifecycle and metering.

## Data model

```sql
CREATE TABLE test_suites (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    config JSONB NOT NULL,                  -- suite-kind-specific
    target_workspace_id UUID,                -- mock to test against (could differ from owner workspace)
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE test_runs (
    id UUID PRIMARY KEY,
    suite_id UUID NOT NULL REFERENCES test_suites(id),
    org_id UUID NOT NULL REFERENCES organizations(id),
    triggered_by TEXT NOT NULL,             -- 'manual' | 'schedule' | 'ci' | 'webhook'
    triggered_by_user UUID REFERENCES users(id),
    status TEXT NOT NULL,                   -- 'queued' | 'running' | 'passed' | 'failed' | 'cancelled' | 'errored'
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    runner_seconds INT,
    summary JSONB,                          -- pass/fail counts, p50/p99, etc.
    git_ref TEXT,                           -- when triggered from CI
    git_sha TEXT
);
CREATE INDEX test_runs_suite_finished_idx ON test_runs (suite_id, finished_at DESC);

CREATE TABLE test_run_events (
    id BIGSERIAL PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES test_runs(id) ON DELETE CASCADE,
    seq INT NOT NULL,                       -- ordering within a run
    event_type TEXT NOT NULL,               -- 'step_start' | 'step_pass' | 'step_fail' | 'log' | 'metric'
    payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE test_schedules (
    id UUID PRIMARY KEY,
    suite_id UUID NOT NULL REFERENCES test_suites(id) ON DELETE CASCADE,
    cron TEXT NOT NULL,                     -- "0 2 * * *"
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_triggered_at TIMESTAMPTZ
);

CREATE TABLE test_run_artifacts (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES test_runs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    content_type TEXT NOT NULL,
    storage_url TEXT NOT NULL,              -- blob storage (Fly volumes / S3)
    size_bytes BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Worker

New crate `mockforge-test-runner` (or extend `mockforge-runtime-daemon`) that:
- Pulls a job from `runner_jobs` queue.
- Downloads suite config, target workspace credentials, fixtures.
- Dispatches to the right executor based on `kind` (k6, conformance, integration, OWASP).
- Streams events to the registry via `POST /api/v1/test-runs/{id}/events` (internal, mTLS-authed).
- Uploads artifacts on completion.
- Reports final status + `runner_seconds`.

Deployment: build a Docker image with k6 + mockforge binaries; Fly machine pool sized to org's plan tier.

## Plan tiers

- **Free**: no test execution.
- **Pro**: 500 runner-minutes / month, max 1 concurrent run.
- **Team**: 5000 runner-minutes / month, max 3 concurrent runs.
- **Enterprise**: 50000 runner-minutes baseline, max 10 concurrent runs, custom on-prem runner option.
- **Top-up packs**: same model as AI Studio — buy additional runner-minutes valid for the billing period.

## CI integration

```
$ mockforge test run \
    --suite-id <uuid> \
    --token $MOCKFORGE_API_TOKEN \
    --git-ref $GITHUB_REF \
    --git-sha $GITHUB_SHA \
    --wait

[queued]
[running] preparing target...
[step] GET /users -> 200 OK (12ms)
[step] POST /users -> 201 Created (15ms)
[passed] 42 steps in 3.2s · runner-seconds: 8
```

CLI exits non-zero on suite failure; output is human-readable by default, `--format=json` for machine consumption.

## UI changes

1. `AppShell.tsx:217` — add `'testing'`, `'test-generator'`, `'test-execution'`, `'integration-test-builder'`, `'conformance'` to `cloudNavItemIds`.
2. **Test Execution Dashboard**: rewrite to show org-wide run history with suite/workspace/status filters. Live-tail SSE for the in-progress run.
3. **Suite editor**: cloud-mode posts to `POST /api/v1/workspaces/{id}/test-suites` instead of writing to local fixture files.
4. **Schedule editor**: add cron-builder UI per suite.
5. **Run detail page**: timeline view (events table), artifacts tab (download HTML reports), diff vs. previous run.
6. **Quota indicator**: "Used 320 / 500 runner-minutes this month."
7. **Test generator** continues to use AI Studio (#1) — same cloud routing.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Schema migration (suites, runs, events, schedules, artifacts) | ~1.5 days |
| 2 | Suite + run CRUD handlers + queue enqueue | ~2 days |
| 3 | Worker crate (k6 + conformance + integration executors) | ~4 days |
| 4 | On-demand Fly machine spawn + lifecycle | ~2 days |
| 5 | Live event stream (SSE + Redis pubsub) | ~1.5 days |
| 6 | Schedule worker (cron evaluator) | ~1 day |
| 7 | Runner-minute metering + plan-limit enforcement | ~1 day |
| 8 | UI rewrite for cloud mode | ~3 days |
| 9 | CLI integration + auth | ~1.5 days |
| 10 | E2E coverage (queue → run → artifacts → metering) | ~2 days |

Total: ~19 working days for v1. **This is the largest task in the cloud-enablement plan.**

## Decisions

### On-demand Fly machines vs. dedicated runners

**Decision: on-demand for v1.** Fly's billing is per-second; idle dedicated runners would burn money. Cold-start latency (~3-5s) is acceptable for test runs that average minutes. Revisit if customers complain about queue time on heavy concurrent loads.

### One billing meter for all suite kinds

**Decision: yes — runner-seconds.** Don't try to price unit tests differently from k6 load tests. Wall-clock is the resource cost, regardless of what's running. Keeps the pricing page simple.

### CLI is part of mockforge-cli, not a separate binary

**Decision: extend existing `mockforge-cli`.** Adds `mockforge test run` subcommand; reuses auth via `~/.mockforge/credentials.json`. One binary to install.

## Out of scope for v1

- Custom runner images (everyone gets the official one).
- BYOI runners (bring-your-own-infra) — Enterprise feature, defer.
- Test result trend charts (only previous-run diff in v1).
- Flaky-test detection / quarantine.
- Visual regression / screenshot diff tests.

## Open questions

1. Do scheduled runs count against runner-minutes the same way manual runs do? Probably yes, but it should be obvious in the UI so users don't get surprise overages.
2. Should test runs against hosted-mocks be free (since the mock is already paid for)? Tempting but breaks the pricing-simplicity rule. Recommend keeping it metered.
3. Where do test-run artifacts live? Fly volumes are simple but tied to a region; S3 is portable but adds an integration. Probably Fly to start, port to S3 if customers ask for cross-region.
