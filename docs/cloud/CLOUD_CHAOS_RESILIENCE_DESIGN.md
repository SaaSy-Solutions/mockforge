# Cloud Chaos + Resilience — Design

Cloud-enablement plan for `chaos` and `resilience` nav items. Tracks task #7 in the cloud-enablement plan.

## Goal

Move chaos engineering and resilience testing to a managed-campaign product. Customers schedule chaos campaigns from the cloud against their own services or their hosted mocks; results are stored as reports with regression diffing. Pricing dial: campaign-minutes consumed and concurrent-campaign caps.

## What exists

- **`mockforge-chaos`** is a large crate with many pieces already shaped for cloud:
  - `gitops.rs`, `multi_tenancy.rs`, `multi_cluster.rs`, `distributed_coordinator.rs` — multi-tenant primitives.
  - `scenario_scheduler.rs`, `scenario_orchestrator.rs`, `scenario_recorder.rs`, `scenario_replay.rs` — scheduled scenarios.
  - `resilience.rs`, `resilience_api.rs` — resilience patterns.
  - `analytics.rs`, `advanced_analytics.rs`, `dashboard.rs`, `observability_api.rs` — reporting.
  - `recommendations.rs`, `ml_anomaly_detector.rs`, `predictive_remediation.rs` — ML add-ons.
  - `template_marketplace.rs` — chaos scenario templates.
- **`mockforge-route-chaos`** is a thin per-route injection helper.
- **UI pages**: `ChaosPage`, `ResiliencePage` (local-only).
- **Zero registry routes for chaos** — none of the in-crate APIs are reached over the cloud admin surface today.

The crate has more infrastructure than we need for v1 cloud — most of it should stay as a library and the cloud surface should be a thin façade over the most important capabilities.

## What's missing (cloud surface)

1. **Campaign as a first-class persisted resource.** Today campaigns live in code/config; cloud needs CRUD with versioned configs.
2. **Run orchestration.** Reuse the #4 Test Execution worker pool — chaos campaigns are just another suite kind. Add `kind = 'chaos_campaign'`.
3. **Target binding.** A campaign needs a target to inject chaos into:
   - Cloud-hosted mock (in-process injection, recorded in the mock's runtime metrics).
   - External service (the worker proxies through chaos middleware → user's URL).
4. **Reports.** Each run produces a campaign report: timeline, fault distribution, observed effects (latency, errors), recommendations.
5. **Resilience patterns library.** `resilience.rs` patterns (circuit breakers, retries, bulkheads) need to be exposable as configurable templates.
6. **Schedules.** Reuse #4's `test_schedules` table — a chaos campaign can be cron-scheduled.
7. **Safety guardrails.** Cloud-initiated chaos against a customer's prod is dangerous. Need:
   - Explicit "I authorize chaos against {target}" flag per run.
   - Automatic kill-switch if target error rate exceeds a threshold.
   - Rate-of-change cap so we don't escalate fault intensity too quickly.

## Cloud architecture

```
[ User defines campaign in UI ]
            │
            ▼
   POST /api/v1/workspaces/{id}/chaos-campaigns
            │
            ▼
        chaos_campaigns row
            │
   ┌────────┴─────────┐
   │                  │
   ▼                  ▼
Manual run     Schedule (cron)
   │                  │
   └────────┬─────────┘
            ▼
   Reuse #4 test_runs (kind='chaos_campaign')
            │
            ▼
   Worker injects faults via mockforge-chaos library
            │
            ├── Streams events ──▶ Postgres + Redis pubsub (same as test runs)
            │
            └── On exit: report → chaos_campaign_reports row
```

### Proposed routes

```
GET    /api/v1/workspaces/{workspace_id}/chaos-campaigns
POST   /api/v1/workspaces/{workspace_id}/chaos-campaigns
GET    /api/v1/chaos-campaigns/{id}
PATCH  /api/v1/chaos-campaigns/{id}
DELETE /api/v1/chaos-campaigns/{id}

POST   /api/v1/chaos-campaigns/{id}/runs                  # trigger run (uses test_runs lifecycle)
POST   /api/v1/chaos-campaigns/{id}/runs/{run_id}/abort   # kill switch
GET    /api/v1/chaos-campaigns/{id}/reports               # list reports
GET    /api/v1/chaos-campaign-reports/{id}

# Resilience pattern templates
GET    /api/v1/resilience-patterns                        # platform-provided library
GET    /api/v1/workspaces/{workspace_id}/resilience-patterns # user customizations
POST   /api/v1/workspaces/{workspace_id}/resilience-patterns

# Templates marketplace tie-in (already cloud)
GET    /api/v1/chaos-templates                           # browse marketplace templates
POST   /api/v1/workspaces/{workspace_id}/chaos-campaigns/from-template/{template_id}
```

## Data model

```sql
CREATE TABLE chaos_campaigns (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    description TEXT,
    target_kind TEXT NOT NULL,                  -- 'hosted_mock' | 'external'
    target_ref TEXT NOT NULL,                   -- deployment_id or URL
    config JSONB NOT NULL,                      -- fault types, intensities, schedule
    safety_config JSONB NOT NULL,               -- kill-switch thresholds
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE chaos_campaign_reports (
    id UUID PRIMARY KEY,
    campaign_id UUID NOT NULL REFERENCES chaos_campaigns(id) ON DELETE CASCADE,
    run_id UUID NOT NULL REFERENCES test_runs(id),
    fault_count INT NOT NULL,
    aborted BOOLEAN NOT NULL DEFAULT FALSE,
    abort_reason TEXT,
    summary JSONB,                              -- p50/p99 latency before/during/after, error rates, etc.
    recommendations JSONB,                      -- from recommendations.rs
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE resilience_patterns (
    id UUID PRIMARY KEY,
    workspace_id UUID REFERENCES workspaces(id),  -- NULL = platform-provided
    kind TEXT NOT NULL,                         -- 'circuit_breaker' | 'retry' | 'bulkhead' | 'rate_limit'
    name TEXT NOT NULL,
    config JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Plan tiers

- **Free**: no chaos campaigns (free users can read scenario marketplace though).
- **Pro**: 200 campaign-minutes / month (reuses test runner pool quota), max 1 concurrent campaign, in-mock targets only (no external).
- **Team**: 2000 campaign-minutes, 3 concurrent, external targets allowed.
- **Enterprise**: 20000 baseline, 10 concurrent, custom safety thresholds, dedicated worker pool option.

Campaign-minutes are billed against the same `runner_seconds_used` counter as test runs — no separate meter.

## Safety guardrails

Cloud chaos against a customer's external service is high-blast-radius. Three layers:

1. **Pre-run authorization.** Each external target requires a one-time URL ownership proof (DNS TXT or HTTP `/.well-known/mockforge-chaos-authorized` file). Cached for 30 days.
2. **Kill switch.** Each run carries a `safety_config` with `max_target_error_rate`, `max_p99_latency_ms`, `target_check_interval_s`. Worker polls target health; trips → abort run, mark `aborted=true`.
3. **Intensity ramp cap.** Fault intensity (e.g., latency injection ms, error rate %) cannot increase by more than X% per minute. Forces gradual ramps, gives ops a chance to notice.

Hosted-mock targets get relaxed guardrails (the customer owns the mock; blast radius is contained).

## UI changes

1. `AppShell.tsx:217` — add `'chaos'`, `'resilience'` to `cloudNavItemIds`.
2. **ChaosPage rewrite**:
   - Campaign list with target/status/schedule columns.
   - Campaign editor: fault types (latency, error rate, network partition), intensity, duration, schedule, safety thresholds.
   - Live run view: fault timeline, target metrics, abort button.
   - Report viewer: before/during/after charts, recommendations.
3. **ResiliencePage rewrite**:
   - Patterns library (platform + user-defined).
   - "Test pattern in a campaign" deep-link to chaos editor.
4. **Templates browser** (already cloud via #11 — `template-marketplace`): adds a chaos category.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Schema migration (campaigns, reports, patterns) | ~1 day |
| 2 | Campaign CRUD + safety_config validation | ~2 days |
| 3 | New worker `kind = 'chaos_campaign'` reusing #4 runner pool | ~2 days |
| 4 | Target authorization (DNS TXT + well-known proof) | ~2 days |
| 5 | Kill-switch worker (target health poll + abort) | ~1.5 days |
| 6 | Report generation + recommendations integration | ~1.5 days |
| 7 | Resilience patterns CRUD | ~1 day |
| 8 | UI rewrites | ~3 days |
| 9 | E2E (campaign → run → kill switch → report) | ~1.5 days |

Total: ~15 working days for v1 (assumes #4 worker pool exists).

## Decisions

### Reuse Test Execution worker pool

**Decision: yes.** Don't build a parallel chaos runner fleet. New `kind` on the same runners. Same metering bucket too.

### Don't expose the heavy ML modules in v1 cloud

**Decision: defer ML modules** (`ml_anomaly_detector`, `ml_parameter_optimizer`, `multi_armed_bandit`, `reinforcement_learning`, `predictive_remediation`). They look impressive in the crate but adding cloud surface for them triples the design surface for unclear customer demand. Ship the basics first; revisit ML modules as a paid Enterprise add-on if customers ask.

### Hosted-mock chaos vs. external chaos as one product

**Decision: same product, different guardrails.** Same campaign editor for both target kinds; only the safety wrapper changes. Avoids "chaos for hosted" and "chaos for external" feeling like two products.

## Out of scope for v1

- ML-based anomaly detection / auto-remediation modules.
- Multi-cluster coordination across customer regions.
- Chaos as a continuous mode (always-on background fault injection).
- Game-day mode (multi-team coordinated chaos sessions).
- Chaos against gRPC / WebSocket / Kafka / MQTT in cloud (HTTP only for v1).

## Open questions

1. The crate has `template_marketplace.rs` *inside* `mockforge-chaos` — likely overlaps with the cloud `template-marketplace` page. Need to figure out which is canonical before building cloud chaos templates.
2. External target authorization via DNS-TXT is annoying for short-lived campaigns. Should we support a header-based proof (request includes a one-time signature) for short campaigns? Probably yes for Pro+, with a 1-hour TTL.
3. Resilience patterns currently live in code as Rust modules. Cloud version persists them as JSON configs — does the runtime know how to apply a JSON-defined retry policy? Need to check `resilience.rs` for whether it accepts data-driven config or only typed.
