# Cloud Observability Stack — Design

Cloud-enablement plan for the observability nav items (`observability`, `logs`, `traces`, `metrics`, `analytics`). Tracks task #2 in the cloud-enablement plan.

## Goal

Move observability from local-only to cloud as a Datadog/New-Relic-shaped tier: cross-deployment queries, team dashboards, retention windows as the pricing dial. Pillar analytics is already cloud — this picks up the rest.

## What already exists in cloud

A surprising amount of ingest infrastructure is in place, scoped per hosted-mock deployment:

- `runtime_logs`, `runtime_traces`, `runtime_captures` tables (migrations `20250101000051..53`).
- Ingest: `POST /api/v1/hosted-mocks/{deployment_id}/runtime-logs/ingest`, `POST /api/v1/hosted-mocks/{deployment_id}/otlp/v1/traces`.
- Query: `GET /api/v1/hosted-mocks/{deployment_id}/runtime-logs[/stream]`, `GET .../traces[/{trace_id}]`, `GET .../metrics`.
- `/api/v1/dashboard/logs` aggregates across the org.
- `/api/v1/admin/analytics`, `/api/v1/admin/analytics/funnel` (admin-only today).
- Pillar analytics is fully cloud-enabled at org and workspace scope.

## What's missing

1. **Cross-deployment org-scoped queries.** The nav pages (Logs, Traces, Metrics) want "show me everything across my workspaces." Today every endpoint requires a `deployment_id`. We need:
   - `GET /api/v1/organizations/{org_id}/logs` with filters: `workspace_id`, `deployment_id`, `level`, `path_pattern`, `from`, `to`.
   - `GET /api/v1/organizations/{org_id}/logs/stream` — SSE/WebSocket live tail across all org deployments.
   - `GET /api/v1/organizations/{org_id}/traces` with workspace/deployment filters.
   - `GET /api/v1/organizations/{org_id}/metrics` (rollups by workspace, deployment, time bucket).
2. **Local-source ingest.** A user running MockForge locally has no way to ship logs/traces/metrics to the cloud. Two options:
   - **(a) Local agent push:** `mockforge-observability::log_shipper` already exists — extend it to target the cloud ingest endpoint when authenticated. Auth via API token.
   - **(b) OSS-only / hosted-only:** ignore local sources in cloud — only show hosted-mock data.
   - Recommend **(a)** behind a `--cloud-ship` flag so OSS users opt in. Lets the cloud Logs page work for users who haven't migrated to hosted mocks yet — important hook for upselling.
3. **Retention tiers.** Today there's a single retention loop (`mockforge-analytics::retention`). Cloud needs plan-based retention:
   - Free: 24h (or no observability at all — leans into cloud-as-paid-feature).
   - Pro: 7 days.
   - Team: 30 days.
   - Enterprise: 90 days, custom on request.
   - Implement as a per-plan TTL applied during the retention sweep; older rows pruned by a background worker.
4. **Storage quotas (logs/traces volume).** Even with retention, runaway log volume is expensive. Add a `usage_counters.log_bytes_ingested` column (mirror of `ai_tokens_used`) and meter at ingest time. Soft-warn at 80%, drop with `429` at 100%.
5. **Dashboards / saved queries.** The Analytics page today is a fixed dashboard. Cloud pricing leverage comes from saved searches and per-workspace dashboards. New tables:
   - `observability_dashboards (id, org_id, workspace_id, name, layout, queries, created_by, created_at, updated_at)`.
   - `observability_saved_queries (id, org_id, name, type, filters, created_by, created_at)`.
   Defer pretty dashboard editor to v2; v1 ships fixed layouts but saves filter sets.
6. **Alerting.** Already covered by the Incidents task (#3) — alerts are powered by saved queries here. Pulled out of v1 scope.

## Cloud architecture

### Ingest paths

```
[ Local mockforge ]──(opt-in --cloud-ship + API token)──┐
                                                         ▼
[ Hosted-mock container ]──(existing ingest endpoints)──▶ runtime_logs / runtime_traces / runtime_captures
                                                         │
                                                         ▼
                                              [ Retention worker ]
                                              prunes rows past plan TTL
```

### Query paths

```
UI (cloud mode) ──▶ /api/v1/organizations/{org_id}/{logs|traces|metrics}
                       │
                       ▼
                 Filter by org_id (auth middleware) + optional workspace/deployment
                       │
                       ▼
                 Postgres (runtime_logs etc.) + Redis (last-N hot cache for stream)
```

### New routes

```
GET  /api/v1/organizations/{org_id}/logs
GET  /api/v1/organizations/{org_id}/logs/stream                 # SSE
GET  /api/v1/organizations/{org_id}/traces
GET  /api/v1/organizations/{org_id}/traces/{trace_id}
GET  /api/v1/organizations/{org_id}/metrics
GET  /api/v1/organizations/{org_id}/observability/saved-queries
POST /api/v1/organizations/{org_id}/observability/saved-queries
GET  /api/v1/organizations/{org_id}/observability/dashboards
POST /api/v1/organizations/{org_id}/observability/dashboards
PATCH /api/v1/organizations/{org_id}/observability/dashboards/{id}

# Local-source ingest (auth via org API token, not deployment_id)
POST /api/v1/organizations/{org_id}/observability/logs/ingest
POST /api/v1/organizations/{org_id}/observability/otlp/v1/traces
```

## Data model changes

- `runtime_logs.workspace_id`, `runtime_traces.workspace_id` — denormalize so we can filter without joining `hosted_deployments`. The lookup happens once at ingest.
- `runtime_logs.source` (`hosted` | `local`) — distinguishes hosted-mock logs from locally-shipped logs.
- `usage_counters.log_bytes_ingested BIGINT NOT NULL DEFAULT 0`.
- Plan-limits config: add `log_retention_days`, `log_bytes_per_month` to the plan-limits JSON already used by `effective_limits` in `handlers/usage.rs`.
- New tables: `observability_dashboards`, `observability_saved_queries` (see above).

## UI changes

1. `AppShell.tsx:217` — add `'observability'`, `'logs'`, `'traces'`, `'metrics'`, `'analytics'` to `cloudNavItemIds`.
2. `services/api/logs.ts`, `services/api/metrics.ts`, new `services/api/traces.ts` — switch base URL using `isCloudMode()`. Cloud paths use `/api/v1/organizations/{org_id}/...`.
3. **Workspace/deployment filter dropdowns** at the top of each page (logs, traces, metrics) — needed because cloud mode is org-scoped, local was single-instance.
4. **Retention/quota indicator** in page header: "Retaining last 7 days · 4.2 GB / 10 GB used."
5. **Saved-query menu** in Logs and Traces pages.
6. **Local-source toggle** on Logs page — let users include/exclude logs shipped from their local machine.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Schema migration (denormalized workspace_id, source col, log_bytes_ingested) | ~1 day |
| 2 | Org-scoped query handlers + auth scoping | ~2 days |
| 3 | Live-tail SSE endpoint + Redis hot cache | ~1.5 days |
| 4 | Local-source ingest endpoint + log_shipper wiring + `--cloud-ship` flag | ~1.5 days |
| 5 | Plan-based retention worker | ~1 day |
| 6 | Saved queries + dashboards CRUD | ~2 days |
| 7 | UI cloud-mode wiring, filter dropdowns, retention indicator | ~2 days |
| 8 | E2E coverage (ingest → query → retention prune) | ~1 day |

Total: ~12 working days for v1 (notably longer than AI Studio because the data layer needs work).

## Decisions

### Retention as the pricing dial vs. ingestion volume

**Decision: bill on both, weighted toward retention.** Storage scales with `bytes × days_retained`, so retention is the natural lever; ingestion volume is the abuse-prevention lever. Free tier doesn't get observability at all — pushes the upgrade.

### Local-source ingest as opt-in only

**Decision: require `--cloud-ship` flag explicitly.** Don't auto-ship logs from local instances; that surprises users (PII concerns, network egress). Opt-in keeps OSS users in control.

### Hot-path cache

**Decision: Redis-backed last-N (e.g., 1000 entries) per (org, log/trace) for live tail.** Avoids hammering Postgres for the SSE stream. Cold-path queries hit Postgres directly.

## Out of scope for v1

- Dashboard layout editor (ship fixed layouts; users save filters only).
- Custom retention per-workspace (org-level only).
- Cross-org queries for admin/support roles (use existing `/api/v1/admin/*` paths).
- Log parsing / structured field extraction beyond what OTLP already provides.
- Alerting (covered by #3 Incidents).

## Open questions

1. Do we surface raw OTLP metrics in the Metrics page, or pre-aggregate into time buckets server-side? Pre-agg is faster but less flexible.
2. Should local-shipped logs count toward the same `log_bytes_per_month` quota as hosted-mock logs? Probably yes (same storage cost), but the distinction may matter for pricing copy.
3. Free tier: zero observability, or 24h retention as a tease? 24h is friendlier; zero is a stronger upgrade prompt.
