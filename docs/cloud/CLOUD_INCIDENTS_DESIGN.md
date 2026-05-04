# Cloud Incidents — Design

Cloud-enablement plan for the `incidents` nav item. Tracks task #3 in the cloud-enablement plan.

## Goal

Move incidents from local-only to cloud as the unified "what just fired" surface — drift incidents, observability alerts, contract verification failures, hosted-mock health alerts. Includes notification routing (email, Slack, PagerDuty, generic webhook) and an on-call-style escalation policy. Pricing dial: number of notification channels and retention of incident history.

## What exists

- **Local UI**: `IncidentDashboardPage.tsx` is a *drift-incident* dashboard backed by `services/driftApi.ts`. Includes severity/status filters, ack/resolve actions, consumer-impact panels.
- **Cloud surface**: `handlers/status.rs` derives a transient `incidents: Vec<Incident>` from current service health for the public status page. No persistence, no acknowledgment.
- **Adjacent cloud features**: hosted-mock health checks (`status` page), audit logs.

There is **no real incident management system in cloud today**. The drift incidents in local UI are computed from contract-validation results in-memory.

## Cloud architecture

### What's missing

1. **Persistent incident model.** Incidents need a row, lifecycle (open → ack → resolved), and an event timeline. Local computes them on the fly; cloud needs durable state.
2. **Incident sources.** Multiple subsystems should be able to *raise* incidents:
   - Contract drift (from #8 Contract diff cloud).
   - Observability alerts (from #2 Observability — saved-query thresholds).
   - Hosted-mock health (already has health checks; today they go to status page only).
   - External webhooks (let users POST their own incidents from CI).
3. **Notification channels.** Per-org config for email recipients, Slack webhook, PagerDuty integration key, generic outbound webhook. None exist today.
4. **Routing rules.** Map (severity × source × workspace) → channel. Even simple "everything critical → PagerDuty, everything else → Slack" is enough for v1.
5. **Acknowledge/resolve API.** With persistence, acks need to flow through cloud not local state.
6. **Postmortem links.** Lightweight: each incident has an optional URL to a runbook/postmortem doc.

### Proposed routes

```
GET    /api/v1/organizations/{org_id}/incidents                       # list with filters
GET    /api/v1/organizations/{org_id}/incidents/{id}
PATCH  /api/v1/organizations/{org_id}/incidents/{id}                  # ack, resolve, assign
POST   /api/v1/organizations/{org_id}/incidents                       # external raise (webhook-style)
GET    /api/v1/organizations/{org_id}/incidents/{id}/events           # timeline

GET    /api/v1/organizations/{org_id}/notification-channels
POST   /api/v1/organizations/{org_id}/notification-channels
PATCH  /api/v1/organizations/{org_id}/notification-channels/{id}
DELETE /api/v1/organizations/{org_id}/notification-channels/{id}
POST   /api/v1/organizations/{org_id}/notification-channels/{id}/test # send test notification

GET    /api/v1/organizations/{org_id}/routing-rules
POST   /api/v1/organizations/{org_id}/routing-rules
PATCH  /api/v1/organizations/{org_id}/routing-rules/{id}
DELETE /api/v1/organizations/{org_id}/routing-rules/{id}
```

### Internal raise path (no public route)

Other handlers fire incidents through an internal `IncidentBus` trait:

```rust
trait IncidentBus {
    async fn raise(&self, input: RaiseIncidentInput) -> Result<IncidentId>;
    async fn resolve(&self, source: IncidentSource, dedupe_key: &str) -> Result<()>;
}
```

`dedupe_key` collapses repeat fires of the same condition into a single open incident — important for noisy drift checks.

## Data model

```sql
CREATE TABLE incidents (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id),
    workspace_id UUID REFERENCES workspaces(id),
    source TEXT NOT NULL,            -- 'drift' | 'observability' | 'hosted_mock_health' | 'external'
    source_ref TEXT,                  -- e.g., contract_diff_run_id
    dedupe_key TEXT NOT NULL,
    severity TEXT NOT NULL,           -- 'critical' | 'high' | 'medium' | 'low'
    status TEXT NOT NULL,             -- 'open' | 'acknowledged' | 'resolved'
    title TEXT NOT NULL,
    description TEXT,
    postmortem_url TEXT,
    assigned_to UUID REFERENCES users(id),
    acknowledged_at TIMESTAMPTZ,
    acknowledged_by UUID REFERENCES users(id),
    resolved_at TIMESTAMPTZ,
    resolved_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX incidents_open_dedupe_idx
    ON incidents (org_id, source, dedupe_key)
    WHERE status != 'resolved';

CREATE TABLE incident_events (
    id UUID PRIMARY KEY,
    incident_id UUID NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,         -- 'created' | 'acknowledged' | 'commented' | 'resolved' | 'reopened' | 'notification_sent'
    actor_id UUID REFERENCES users(id),
    payload JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE notification_channels (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id),
    name TEXT NOT NULL,
    kind TEXT NOT NULL,               -- 'email' | 'slack' | 'pagerduty' | 'webhook'
    config JSONB NOT NULL,            -- encrypted secrets via existing settings::encrypt
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE routing_rules (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id),
    priority INT NOT NULL,
    match_severity TEXT[],            -- empty = match all
    match_source TEXT[],
    match_workspace_id UUID,
    channel_ids UUID[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

Reuse existing `settings::encrypt_api_key` for channel secrets (Slack URLs, PD keys).

## Notification dispatch

Background worker (`mockforge-registry-server::workers::incident_dispatcher`) consumes new incidents off a Redis queue and fans out to matched channels. Existing `email.rs` covers the email channel; Slack/PagerDuty/webhook are HTTP POSTs with templated payloads. Each dispatch attempt logs an `incident_event` of type `notification_sent`.

Failure handling: 3 retries with exponential backoff per channel, then mark the dispatch failed in the timeline. Don't block other channels on one failure.

## UI changes

1. `AppShell.tsx:217` — add `'incidents'` to `cloudNavItemIds`.
2. **Incidents page rewrite**: today it's drift-only. In cloud mode, show all sources with a source filter. Severity/status filters stay.
3. **Incident detail drawer**: add timeline view, ack/resolve buttons, assign-to dropdown, postmortem link field.
4. **New Notification Channels page** under Configuration (`config` group): list, add, test, delete channels.
5. **New Routing Rules editor**: drag-to-reorder priority list, severity/source/workspace match conditions, channel multi-select.
6. **Test-fire button**: lets admins send a synthetic incident through the routing rules to verify config.

## Plan tiers (pricing dial)

- **Free**: no incidents (read-only status page only).
- **Pro**: incidents enabled, max 2 notification channels, 30 days retention, no PagerDuty channel.
- **Team**: 10 channels, 90 days retention, PagerDuty included.
- **Enterprise**: unlimited channels, 1 year retention, custom routing rules with on-call schedules.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Schema migration (incidents, events, channels, routing) | ~1 day |
| 2 | CRUD handlers + IncidentBus trait + dedupe logic | ~2.5 days |
| 3 | Notification dispatcher worker (email/Slack/webhook); PagerDuty deferred | ~2 days |
| 4 | Routing rule evaluator | ~1 day |
| 5 | UI: incidents page rewrite + detail drawer | ~2 days |
| 6 | UI: notification channels + routing rules editors | ~2 days |
| 7 | Wire #2 (Observability alerts) and #8 (Contract drift) to IncidentBus | ~1.5 days |
| 8 | E2E coverage (raise → dispatch → ack → resolve) | ~1 day |

Total: ~13 working days for v1.

## Decisions

### Dedupe by source + key, not by hash of payload

**Decision: each incident source defines its own `dedupe_key`** (e.g., contract drift uses `endpoint:method`; observability alert uses `saved_query_id`). This keeps the noise-collapse logic in the source's domain rather than centralized hashing that would need to know each source's semantics.

### Channels are org-scoped, not workspace-scoped

**Decision: org-level for v1.** Workspace-scoped channels would be flexible but doubles the config surface. Routing rules already let you scope by workspace, so per-channel scoping is redundant.

### No on-call schedules in v1

**Decision: skip rotations / schedules.** PagerDuty integration covers users who need it; building our own on-call scheduler is a deep rabbit hole and not core to MockForge's value. Revisit if Enterprise customers ask.

## Out of scope for v1

- On-call schedules / rotations.
- Mobile push notifications (use email or Slack mobile).
- Rich incident comments / collaboration (single description field for now).
- Auto-resolve based on signal recovery (manual resolve only; sources can still call `resolve()` programmatically).
- SLA tracking / time-to-acknowledge metrics.

## Open questions

1. PagerDuty in v1 or v2? It's the most-requested integration but adds testing complexity (their API requires real keys). Recommend v2 unless an early customer asks.
2. Should drift incidents reuse the existing `driftApi` shape so the local UI keeps working in self-hosted mode, or unify on the new generic `incidents` shape and have local mode also persist them? Probably unify — less code paths.
3. Free tier with read-only status page only is harsh; should it at least see a count of incidents so the upgrade prompt makes sense?
