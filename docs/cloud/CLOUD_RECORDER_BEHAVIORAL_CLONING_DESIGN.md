# Cloud Recorder + Behavioral Cloning — Design

Cloud-enablement plan for `recorder` and `behavioral-cloning` nav items. Tracks task #6 in the cloud-enablement plan.

## Goal

Move recording and behavioral cloning to a managed-capture-and-train product. Customers record real traffic to cloud storage, then train cloned mock behaviors from the captures — both billed by data volume and training-job runtime. The recording side is mostly already built; behavioral cloning needs cloud worker integration.

## What exists

- **`mockforge-recorder` crate** is large and already has substantial cloud sync infra:
  - `cloud_sync.rs` ships captures from hosted-mock containers to cloud (`#234 part 2`).
  - `sync_drift.rs`, `sync_gitops.rs`, `sync_snapshots.rs`, `sync_traffic.rs` — multiple sync flavors.
  - `behavioral_cloning/` submodule alongside the standalone `mockforge-behavioral-cloning` crate.
- **Registry routes** (deployment-scoped):
  - `POST /api/v1/hosted-mocks/{deployment_id}/captures/ingest`.
  - `GET /api/v1/hosted-mocks/{deployment_id}/captures[/{capture_id}[/response]]`.
  - `GET .../captures/export/{har,jsonl}`.
- **Tables**: `runtime_captures` already persists exchanges past container restart.
- **UI**: `RecorderPage`, `BehavioralCloningPage` (local-only).
- **Multiple behavioral-cloning implementations**: `mockforge-behavioral-cloning`, `mockforge-recorder::behavioral_cloning`, `mockforge-intelligence::behavioral_cloning`. The cloud version should pick one (probably the standalone crate) and delete the others.

## What's missing

1. **Org-scoped capture queries.** Today everything is per-deployment. Same pattern as Observability — add cross-deployment views.
2. **Local-source capture ingest.** A locally-running recorder has no cloud target except hosted-mocks. Mirror the observability `--cloud-ship` flag here.
3. **Capture collections.** Group related captures into named "sessions" (e.g., "checkout-flow-2024-12") for replay/training. New table.
4. **Behavioral cloning as a cloud job.** Today training runs in-process. Cloud needs:
   - Submit a "train clone from session" job.
   - Run it on the same cloud worker pool from #4 Test Execution (reuse infra).
   - Store the trained model artifact.
   - Deploy the model as a runnable mock (ties to `hosted-mocks`).
5. **Replay endpoints.** Replay a captured session against a target service or against a cloud-hosted mock for regression testing.
6. **Capture-data quota.** New `usage_counters.capture_bytes_stored`. Tier on storage volume + retention days.
7. **Three crates collapsed into one.** Pick `mockforge-behavioral-cloning` as the canonical crate; the other two get removed (likely a separate cleanup PR before this work).

## Cloud architecture

```
[ Local recorder / hosted-mock recorder ]
            │
            │  cloud_sync.rs / new local-source endpoint
            ▼
   POST /api/v1/{org_id|hosted-mocks/{id}}/captures/ingest
            │
            ▼
       runtime_captures (Postgres)
            │
            ├── User groups captures into a session ──▶ capture_sessions table
            │
            ├── User triggers "Train clone" ──▶ enqueue training job (reuse #4 worker pool)
            │                                               │
            │                                               ▼
            │                                      Worker downloads session
            │                                      Trains model → uploads to blob storage
            │                                      Reports clone_models row
            │
            └── Trained clone deployable as a hosted mock ──▶ hosted_deployments
```

### Proposed routes

```
GET    /api/v1/organizations/{org_id}/captures                    # cross-deployment list
GET    /api/v1/captures/{capture_id}
GET    /api/v1/captures/{capture_id}/response

# Local-source ingest (org-scoped, API-token auth)
POST   /api/v1/organizations/{org_id}/captures/ingest

# Sessions (capture collections)
GET    /api/v1/workspaces/{workspace_id}/capture-sessions
POST   /api/v1/workspaces/{workspace_id}/capture-sessions
PATCH  /api/v1/capture-sessions/{id}                              # add/remove captures
DELETE /api/v1/capture-sessions/{id}

# Behavioral cloning
POST   /api/v1/capture-sessions/{id}/train                        # enqueue training job
GET    /api/v1/clone-models                                        # list trained models
GET    /api/v1/clone-models/{id}
POST   /api/v1/clone-models/{id}/deploy                            # deploy as hosted mock
DELETE /api/v1/clone-models/{id}

# Replay
POST   /api/v1/capture-sessions/{id}/replay                        # replay against target URL
GET    /api/v1/replay-runs/{run_id}                                # replay results (uses test_runs from #4)
```

## Data model

```sql
ALTER TABLE runtime_captures ADD COLUMN workspace_id UUID;
ALTER TABLE runtime_captures ADD COLUMN source TEXT NOT NULL DEFAULT 'hosted';  -- 'hosted' | 'local'

CREATE TABLE capture_sessions (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    name TEXT NOT NULL,
    description TEXT,
    capture_count INT NOT NULL DEFAULT 0,
    total_bytes BIGINT NOT NULL DEFAULT 0,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE capture_session_members (
    session_id UUID NOT NULL REFERENCES capture_sessions(id) ON DELETE CASCADE,
    capture_id UUID NOT NULL REFERENCES runtime_captures(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (session_id, capture_id)
);

CREATE TABLE clone_models (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id),
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    source_session_id UUID REFERENCES capture_sessions(id),
    name TEXT NOT NULL,
    status TEXT NOT NULL,                   -- 'training' | 'ready' | 'failed'
    artifact_url TEXT,                       -- blob storage URL for the model
    metrics JSONB,                           -- accuracy, coverage, latency P50/P99 etc.
    runner_seconds INT,
    deployed_to UUID REFERENCES hosted_deployments(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

Add `usage_counters.capture_bytes_stored BIGINT NOT NULL DEFAULT 0`.

## Plan tiers

- **Free**: 100 MB capture storage, 7-day retention, no behavioral cloning.
- **Pro**: 5 GB storage, 30-day retention, 2 trained clones, training counts against runner-minutes (#4).
- **Team**: 50 GB storage, 90-day retention, 10 trained clones, dedicated training queue priority.
- **Enterprise**: custom storage, 1-year retention, unlimited clones, on-prem training option.

## UI changes

1. `AppShell.tsx:217` — add `'recorder'`, `'behavioral-cloning'` to `cloudNavItemIds`.
2. **RecorderPage rewrite for cloud mode**:
   - Cross-deployment capture list with workspace/source/path filters.
   - Multi-select → "Add to session" action.
   - Capture detail drawer: request, response, headers, replay button.
3. **CaptureSessions sub-page**: list sessions, see captures in each, train clone, replay.
4. **BehavioralCloningPage rewrite**:
   - Pulls `clone_models` from cloud.
   - Training-job status with live runner-second meter.
   - Per-model metrics card: coverage, accuracy, sample exchanges.
   - "Deploy as hosted mock" button → calls `POST /clone-models/{id}/deploy`.
5. **Storage-quota indicator** on RecorderPage header.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 0 | Crate consolidation: pick canonical `mockforge-behavioral-cloning`, delete duplicates | ~1 day |
| 1 | Schema migration (workspace_id/source on captures, sessions, clone_models, capture_bytes_stored) | ~1 day |
| 2 | Org-scoped capture query handlers + filters | ~1.5 days |
| 3 | Local-source ingest endpoint + recorder shipper wiring | ~1 day |
| 4 | Capture session CRUD + member management | ~1 day |
| 5 | Training-job enqueue, reuse #4 worker pool with new "behavioral_clone" kind | ~2 days |
| 6 | Clone-model storage + deploy-as-hosted-mock integration | ~2 days |
| 7 | Replay endpoint + reuse test_runs lifecycle | ~1.5 days |
| 8 | Storage-quota metering at ingest | ~0.5 day |
| 9 | UI rewrites for cloud mode | ~3 days |
| 10 | E2E (capture → session → train → deploy → replay) | ~1.5 days |

Total: ~15 working days for v1 (assumes #4 Test Execution worker pool is already done — otherwise add ~5 days).

## Decisions

### Reuse the #4 worker pool for training

**Decision: yes.** Behavioral cloning training is wall-clock-bound work that fits the same on-demand-Fly-machine pattern as test runs. New `kind = 'behavioral_clone'`. Avoids building a parallel worker fleet.

### Storage on Postgres TOAST or external blob?

**Decision: small captures stay in Postgres (`runtime_captures.body` already does this); training-model artifacts go to blob storage.** Captures are queried frequently with filters, so co-location with metadata is fine. Models are write-once / read-rarely and can be large.

### Crate consolidation as a prerequisite

**Decision: do consolidation as a separate cleanup PR before starting cloud work.** Three implementations of the same thing is technical debt that will multiply if cloud work forks again. ~1 day to pick canonical and delete the rest.

## Out of scope for v1

- Real-time clone deployment from a live capture stream (batch sessions only).
- Cross-workspace capture sharing.
- Differential cloning (train clone from "diff between session A and B").
- Privacy-preserving training (PII detection beyond existing `scrubbing.rs`).

## Open questions

1. Captures often contain PII. Existing `scrubbing.rs` handles redaction in the recorder before it ships. Do we re-scrub at the cloud boundary as a defense-in-depth, or trust the client? Recommend re-scrub.
2. Behavioral-cloning models trained on one customer's traffic should never leak across orgs. Storage scoping is in the schema but model artifacts also need org-isolated blob paths.
3. `sync_gitops.rs` suggests Git-based capture sync exists. Does it ship to cloud already, or is it for local Git? If the former, we may need to deprecate or integrate.
