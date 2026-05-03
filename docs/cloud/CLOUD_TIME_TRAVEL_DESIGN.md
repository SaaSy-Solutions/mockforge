# Cloud Time Travel — Design

Cloud-enablement plan for the `time-travel` nav item. Tracks task #10 in the cloud-enablement plan.

## Goal

Move time-travel snapshots and temporal simulation to cloud as a managed-snapshot product. Customers snapshot the full state of a workspace (mocks, scenarios, world state, fixtures) at a point in time, restore on demand, and apply scale-factor temporal simulation against hosted mocks. Pricing dial: snapshot retention duration + storage volume.

## What exists

- **`mockforge-core::time_travel`** (with `cron.rs`): temporal-simulation engine (controllable clock with scale factor, mutation rules, cron simulation).
- **`mockforge-http::time_travel_api`**: HTTP control plane.
- **`mockforge-ui` `time_travel_handlers`**: local admin endpoints under `/__mockforge/time-travel/{status,enable,disable,...}`.
- **UI**: `TimeTravelPage`, `TimeTravelWidget` overlay component, `services/api/timeTravel.ts`.
- **Registry**: zero time-travel routes today.

The local feature combines two capabilities:
1. **Temporal simulation**: virtual clock with scale factor (e.g., "run at 60x to simulate a day in 24 minutes").
2. **Snapshot/restore**: capture and roll back state.

The snapshot half is the natural cloud product. Temporal simulation runs in-process and ships with hosted mocks.

## What's missing

1. **Snapshot persistence in cloud.** Today snapshots live in local SQLite. Cloud needs:
   - A `snapshots` table with metadata.
   - Blob storage for snapshot payloads (workspace state can be large).
   - Per-workspace and per-hosted-deployment snapshots.
2. **Hosted-mock integration.** Snapshot a running hosted mock, restore on a different deployment. Today restore only works in-process.
3. **Snapshot triggers**:
   - Manual via UI button.
   - Scheduled (e.g., "snapshot nightly").
   - On-event (e.g., "snapshot before this chaos campaign").
4. **Retention tiering.** Retention windows tied to plan, with automatic prune of expired snapshots.
5. **Restore as a job.** Reuse the #4 worker pool — restoring a large snapshot is wall-clock work.
6. **Diff between snapshots.** "What changed between snapshot A and snapshot B" — high-value debugging feature.

## Cloud architecture

```
[ User triggers snapshot ]
            │
            ▼
   POST /api/v1/workspaces/{id}/snapshots
            │
            ├── Captures state inline (small) ──▶ snapshots row + blob
            │
            └── Or enqueues snapshot job (large) ──▶ #4 worker
                                                            │
                                                            ▼
                                                  Worker dumps state, uploads to blob
                                                            │
                                                            ▼
                                                  snapshots row updated to 'ready'

[ User triggers restore ]
            │
            ▼
   POST /api/v1/snapshots/{id}/restore
            │
            ▼
   Enqueue restore job (#4 worker)
            │
            ▼
   Worker downloads blob, applies to target workspace/deployment
```

### Proposed routes

```
GET    /api/v1/workspaces/{workspace_id}/snapshots
POST   /api/v1/workspaces/{workspace_id}/snapshots                   # capture now
GET    /api/v1/snapshots/{id}
GET    /api/v1/snapshots/{id}/manifest                               # what's in this snapshot
DELETE /api/v1/snapshots/{id}

POST   /api/v1/snapshots/{id}/restore                                 # restore to source workspace
POST   /api/v1/snapshots/{id}/restore-to/{target_workspace_id}        # restore to different workspace

GET    /api/v1/snapshots/{id}/diff/{other_snapshot_id}                # diff two snapshots

POST   /api/v1/workspaces/{workspace_id}/snapshot-schedules
DELETE /api/v1/snapshot-schedules/{id}

# Hosted-mock specific
POST   /api/v1/hosted-mocks/{deployment_id}/snapshots
POST   /api/v1/hosted-mocks/{deployment_id}/restore-from/{snapshot_id}
```

## Data model

```sql
CREATE TABLE snapshots (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    hosted_deployment_id UUID REFERENCES hosted_deployments(id),
    name TEXT,
    description TEXT,
    triggered_by TEXT NOT NULL,                 -- 'manual' | 'schedule' | 'pre_chaos' | 'pre_restore'
    triggered_by_user UUID REFERENCES users(id),
    status TEXT NOT NULL,                       -- 'capturing' | 'ready' | 'failed' | 'expired'
    storage_url TEXT,                           -- blob storage location
    size_bytes BIGINT,
    manifest JSONB,                             -- what's included: mocks, scenarios, world state, fixtures, ...
    expires_at TIMESTAMPTZ,                     -- driven by plan retention
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    captured_at TIMESTAMPTZ
);
CREATE INDEX snapshots_workspace_created_idx ON snapshots (workspace_id, created_at DESC);
CREATE INDEX snapshots_expires_at_idx ON snapshots (expires_at) WHERE status = 'ready';

CREATE TABLE snapshot_schedules (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    cron TEXT NOT NULL,
    name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_triggered_at TIMESTAMPTZ
);
```

Add `usage_counters.snapshot_bytes_stored BIGINT NOT NULL DEFAULT 0`.

## Snapshot manifest

```json
{
  "version": 1,
  "components": {
    "mocks": { "count": 12, "size_bytes": 4321 },
    "scenarios": { "count": 5, "size_bytes": 1234 },
    "world_state": { "size_bytes": 9876 },
    "fixtures": { "count": 24, "size_bytes": 87654 },
    "captures": { "included": false, "reason": "size_cap_exceeded" }
  },
  "captured_at": "2026-04-15T10:30:00Z",
  "captured_against_version": "0.3.31"
}
```

Manifest is queryable separately so the UI can list "what's in this snapshot" without downloading the whole blob.

## Plan tiers

- **Free**: 3 snapshots / workspace, 7-day retention, 100 MB total storage.
- **Pro**: 50 snapshots, 30-day retention, 5 GB storage, scheduled snapshots.
- **Team**: 500 snapshots, 90-day retention, 50 GB storage, restore-to-different-workspace.
- **Enterprise**: unlimited snapshots, 1-year retention, custom storage, point-in-time restore for hosted mocks.

## Retention worker

Background worker prunes:
- Snapshots past `expires_at` (auto-set from plan retention at capture time).
- Oldest snapshots when org exceeds `snapshot_bytes_stored` quota (with warning email at 80%).

Reuse the same retention worker pattern as #2 Observability.

## Integration with other tasks

- **#7 Chaos**: each chaos campaign run optionally captures a `pre_chaos` snapshot, restorable with one click after the run. This is the killer ergonomic for chaos testing.
- **#9 Scenario/Orchestration**: a scenario run can checkpoint via snapshot at key states.
- **#4 Test Execution**: restore-job runs through the test runner pool.

## UI changes

1. `AppShell.tsx:217` — add `'time-travel'` to `cloudNavItemIds`.
2. **TimeTravelPage rewrite for cloud mode**:
   - Snapshot list with workspace/deployment/triggered-by filters.
   - "Capture now" button with optional name/description.
   - Snapshot detail drawer: manifest, restore button, diff selector.
   - Schedule editor (reuse cron picker from #4).
   - Storage-quota indicator.
3. **TimeTravelWidget** stays as the temporal-simulation control (in-process feature, not cloud-shaped).
4. **Diff viewer**: side-by-side or unified-diff view of snapshot manifests + per-component drilldowns.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Schema migration + blob storage wiring | ~1.5 days |
| 2 | Snapshot capture handler (inline path for small) + worker job (large) | ~2 days |
| 3 | Restore worker + integrity verification | ~2 days |
| 4 | Cross-workspace restore + safety checks | ~1 day |
| 5 | Schedule worker (reuse #4 cron infra) | ~0.5 day |
| 6 | Diff endpoint + manifest comparison | ~1.5 days |
| 7 | Retention worker + quota enforcement | ~1 day |
| 8 | Hooks: pre-chaos snapshot integration with #7 | ~0.5 day |
| 9 | UI rewrite for cloud mode | ~2 days |
| 10 | E2E (capture → list → restore → diff → expire) | ~1.5 days |

Total: ~13.5 working days for v1.

## Decisions

### Inline-capture vs. always-async

**Decision: inline for small (<10 MB) workspaces, async via worker for large.** Most workspaces are small enough that an inline capture (synchronous, returns the snapshot ID immediately) is the better UX. Big workspaces fall back to a worker job and return `status='capturing'`.

### Blob storage location

**Decision: same blob backend as #4 test artifacts and #6 clone models** (Fly volumes for v1, port to S3 later). Avoids running multiple storage adapters.

### Restore is destructive — confirm twice

**Decision: restore requires (1) explicit confirmation, (2) auto-snapshot of current state pre-restore (with `triggered_by = 'pre_restore'`).** Lets users undo a bad restore. Pre-restore snapshot is free (doesn't count against quota for first 24h).

## Out of scope for v1

- Incremental snapshots (full only).
- Snapshot-driven branching (Git-style "fork from this snapshot").
- Cross-region snapshot replication.
- Encrypted-at-rest with customer keys (use platform encryption only).
- Partial restores (single-mock from snapshot) — restore is all-or-nothing in v1.

## Open questions

1. Does the snapshot include captures (#6) and logs (#2)? They can be huge. Recommend default-no, with an opt-in "include captures" flag per snapshot.
2. Scheduled snapshots could overlap with point-in-time backups customers expect from a database. Be explicit in copy: snapshots are MockForge-state, not infrastructure backups.
3. Hosted-mock restore is the high-value feature but also the riskiest (changing a running mock). Should it require a maintenance-window flag? Probably yes.
