# Cloud Scenario Studio + Orchestration — Design

Cloud-enablement plan for `scenario-studio`, `chains`, `state-machine-editor`, `orchestration-builder`, `orchestration-execution`. Tracks task #9 in the cloud-enablement plan.

## Goal

Move scenario authoring and orchestration execution to cloud as a managed-flows product. Customers define multi-step scenarios (state machines, chains, orchestrations), share them across the team, and run them on cloud workers. Pricing dial: stored scenarios + execution-minutes + scheduled-run frequency.

## What exists

- **Pages**: `ScenarioStudioPage`, `ScenarioStateMachineEditor`, `OrchestrationBuilder`, `OrchestrationExecutionView`, `ChainsPage`.
- **Cloud already**: `scenario-marketplace` (browse/install pre-made scenarios), `template-marketplace`. Federation has scenario-activation routes for cross-federation execution.
- **Crates**: `mockforge-scenarios` (manifest, registry, federation_runtime, mockai_integration), `mockforge-pipelines` (pipeline + steps for chained execution).
- **Registry**: marketplace read endpoints + federation activation, but **no authoring or execution endpoints**. Scenarios are typically local YAML files today.

## What's missing

1. **Authored scenarios persisted in cloud.** Today scenarios live as files; cloud needs CRUD with versioning.
2. **Orchestration definitions** as a distinct resource (pipeline of steps with conditions, parallelism, retries).
3. **State machine definitions** persisted with their states/transitions — same pattern as orchestrations but with a different runtime.
4. **Chain definitions** (request chains): the simplest case — sequential HTTP calls with template-variable forwarding.
5. **Execution runtime in cloud.** Reuse the #4 worker pool with new `kind` values: `scenario`, `orchestration`, `chain`, `state_machine`.
6. **Live execution view.** SSE stream of step events, current-state highlights, variables snapshot.
7. **Sharing model.** Workspace-scoped by default; promote-to-marketplace flow already exists for scenarios — extend it to orchestrations and chains.

## Cloud architecture

These four authoring surfaces (scenarios, orchestrations, state machines, chains) all share a common pattern:

```
[ User authors X in editor ]
            │
            ▼
   POST /api/v1/workspaces/{id}/{flows}        # CRUD
            │
            ▼
        flows row (typed by `kind`)
            │
            ├── User runs ──▶ enqueue test_runs (kind = matching value)
            │                     │
            │                     ▼
            │            Worker (#4) executes flow, emits events
            │                     │
            │                     ▼
            │            test_run_events table (reused from #4)
            │
            └── User schedules ──▶ test_schedules (reused from #4)
```

Treating scenarios/chains/orchestrations/state-machines as four kinds of the same `flows` resource keeps the data layer simple and reuses the entire #4 lifecycle (queue, runner pool, events, artifacts, scheduling, billing).

### Proposed routes

```
GET    /api/v1/workspaces/{workspace_id}/flows                     # filter by kind
POST   /api/v1/workspaces/{workspace_id}/flows
GET    /api/v1/flows/{id}                                           # versioned content via ?version=
PATCH  /api/v1/flows/{id}                                           # creates new version
DELETE /api/v1/flows/{id}
GET    /api/v1/flows/{id}/versions

POST   /api/v1/flows/{id}/runs                                      # enqueue execution (returns test_run_id)
POST   /api/v1/flows/{id}/schedules                                 # reuse #4 schedules

# Promotion to marketplace (scenarios already supported; extend)
POST   /api/v1/flows/{id}/publish-to-marketplace
```

`flows.kind` ∈ `{ 'scenario', 'orchestration', 'state_machine', 'chain' }`. Each kind shares CRUD plumbing but has its own `config_schema` validated server-side.

## Data model

```sql
CREATE TABLE flows (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    current_version_id UUID,                    -- FK to flow_versions, nullable to allow circular FK
    is_published_to_marketplace BOOLEAN NOT NULL DEFAULT FALSE,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE flow_versions (
    id UUID PRIMARY KEY,
    flow_id UUID NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
    version_number INT NOT NULL,
    config JSONB NOT NULL,                      -- kind-specific definition
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (flow_id, version_number)
);

ALTER TABLE flows ADD CONSTRAINT flows_current_version_fk
    FOREIGN KEY (current_version_id) REFERENCES flow_versions(id);
```

No new run/event/schedule tables — those are reused from #4 via `test_runs.suite_id` (which we generalize to "flow_id" in this view; either rename the column or keep a polymorphic association).

## Live execution UI

`OrchestrationExecutionView` becomes a generic flow-run viewer:
- **Steps panel**: list each step with status badge (pending, running, passed, failed, skipped).
- **Current state**: highlights the active node in the canvas (state machine view) or the current step (orchestration view).
- **Variables panel**: snapshot of the run's variable state, updated per event.
- **Events log**: tailing list of `test_run_events` filtered to this run.
- **Abort button**: same kill-switch as #7 chaos.

## UI changes

1. `AppShell.tsx:217` — add `'scenario-studio'`, `'chains'`, `'state-machine-editor'`, `'orchestration-builder'`, `'orchestration-execution'` to `cloudNavItemIds`.
2. **Editors stay client-side** — Monaco / canvas authors don't need cloud round-trips. Save/load goes through CRUD.
3. **Run history sidebar** on each editor: last N runs with status; click to open the execution view.
4. **Schedule editor** as a slide-over (reuse the cron picker from #4).
5. **Marketplace-promotion modal**: only available once a flow has at least one passing run.

## Plan tiers

- **Free**: 5 flows total per workspace, max 100 execution-minutes / month, manual runs only.
- **Pro**: 50 flows, 1000 minutes, schedules at hourly granularity.
- **Team**: 500 flows, 10000 minutes, schedules at minute granularity, marketplace publishing.
- **Enterprise**: unlimited flows, custom minutes, federation-wide flow execution.

Execution-minutes meter against the same `runner_seconds_used` counter as #4 test runs.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Schema migration (flows, flow_versions; reuse #4 run tables) | ~1 day |
| 2 | CRUD + per-kind config validators | ~2 days |
| 3 | Worker executors per kind (chain → orchestration → state machine → scenario) | ~3.5 days |
| 4 | Live event stream wiring (reuse #4 SSE) | ~0.5 day |
| 5 | Schedule wiring (reuse #4 schedules with flow_id) | ~0.5 day |
| 6 | UI editor wiring to cloud CRUD | ~3 days |
| 7 | Execution view rewrite for cloud mode | ~2 days |
| 8 | Marketplace-promotion flow | ~1 day |
| 9 | E2E (author → run → schedule → publish) | ~1.5 days |

Total: ~15 working days for v1 (assumes #4 done first).

## Decisions

### Unify four resource types under `flows`

**Decision: yes.** Editor UX differs per kind, but the persistence/run/scheduling lifecycle is identical. One table avoids duplicate plumbing.

### Reuse #4 test_runs vs. dedicated flow_runs

**Decision: reuse.** A "scenario run" and a "test run" are operationally identical: queue → runner → events → artifacts. Different UI presentation, same backend. Saves a parallel system. Either rename `test_runs.suite_id` to a generic `flow_id` (with FK union semantics) or keep it polymorphic by `kind`.

### Versioning is mandatory, not optional

**Decision: every save creates a new flow_version row.** Scenarios shared across teams need rollback ("oh, last week's version worked, revert"). Cheap to implement; high-value safety net.

## Out of scope for v1

- Visual diff between flow versions (show as JSON diff for now).
- Cross-workspace scenario imports beyond marketplace.
- Real-time collaborative editing (lock-based or last-write-wins; defer CRDTs).
- Custom step types via plugin (use built-ins only in v1).

## Open questions

1. The local UI's `ScenarioStudioPage` likely has different state shapes from `OrchestrationBuilder`. Unifying them in cloud mode means refactoring the editor — how aggressive should that be? Recommend keeping editors separate; only the persistence layer is unified.
2. Marketplace publishing is one-way today (publish snapshot, no updates). Should cloud flows track the live link to a published version? Probably yes — let users push updates to marketplace.
3. Should scheduled flow runs share the same `runner_seconds_used` quota as test runs? Yes (same machines, same cost), but UI should label them separately so quota usage is debuggable.
