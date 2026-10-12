# Observability alert incidents

Saved observability queries can carry an optional `filters.alert` rule. The registry evaluates enabled rules every 30 seconds against persisted telemetry and raises incidents through the shared IncidentBus. Existing saved queries without an alert rule retain their behavior.

## Configure a rule

Use the existing authenticated saved-query API:

```http
POST /api/v1/organizations/{org_id}/observability/saved-queries
Content-Type: application/json
```

```json
{
  "name": "Recorded requests returning server errors",
  "kind": "metrics",
  "filters": {
    "kind": "request_count_by_status",
    "window_minutes": 5,
    "alert": {
      "metric": "error_count",
      "threshold": 1,
      "severity": "critical",
      "window_minutes": 5,
      "min_samples": 5,
      "enabled": true
    }
  }
}
```

The dashboard query and alert rule each keep their own window. PATCH `/api/v1/observability/saved-queries/{id}` with a replacement `filters` object to edit the rule. Set `alert.enabled` to false, remove `alert`, or set it to null to stop evaluation. These changes do not silently close a previously raised incident; it remains available for an operator to resolve.

## Signals and scope

| Saved-query kind | Stored signal | Error definition | Latency in milliseconds |
| --- | --- | --- | --- |
| `logs` | Structured request logs (`runtime_request_logs`) | HTTP status >= 500 | Request latency |
| `traces` | OTLP spans (`runtime_traces`) | OTel ERROR status (2) | End minus start timestamp |
| `metrics` | Recorded exchanges (`runtime_captures`) | Response status, falling back to request status, >= 500 | Recorded duration |

Supported metrics are `sample_count`, `error_count`, `error_rate` (a fraction from 0 to 1), and `latency_p95_ms`. A value equal to or greater than the positive threshold breaches the rule. Windows must be 1..1440 minutes. `min_samples` defaults to 1 and must be positive. Error metrics count only observations with a known status; latency metrics count only valid, nonnegative durations. Incomplete responses and invalid durations cannot act as healthy evidence. Missing or insufficient samples, or missing latency for a latency rule, do not prove recovery and leave an existing incident open.

All data is joined to an organization-owned hosted deployment. Optional `alert.deployment_id` restricts the rule to one owned deployment. `alert.service_name` is an exact trace-service filter; `alert.path_contains` is a literal substring filter for logs and captures. Unknown rule fields and incompatible filters are rejected instead of silently ignored. Deployment and workspace ownership are checked through the tenant-scoped API.

An optional saved-query `workspace_id` scopes **metrics/capture** rules to that workspace, whose ownership is checked again by the worker. Logs and traces do not carry workspace IDs; their rules support organization or deployment scope, and workspace-scoped log/trace rules are rejected. Hosted capture ingestion currently does not populate workspace IDs, so workspace rules see only correctly attributed captures. This existing ingestion gap is separate from alert evaluation.

## Incident lifecycle

Severity is translated centrally: `critical` and `high` retain their values, `warning`/`medium` become `medium`, and `info`/`low` become `low`. Incident source is `observability`; the saved-query ID is both its source reference and dedupe key. Repeated breaches retain one unresolved incident. A sampled window below the threshold resolves it and emits the existing resolution timeline event. A later breach can open a new incident. Notifications use the existing incident dispatcher and routing channels.

A PostgreSQL transaction-scoped advisory lock serializes evaluator ticks across replicas. Evaluation and incident changes use one pool connection, including nested IncidentBus savepoints, so a one-connection pool does not deadlock. Per-rule savepoints isolate database failures; failures are logged and other rules continue. Cancellation or an uncommitted tick rolls back incident changes and releases the lock. Notifications become visible after commit. This is periodic evaluation of available telemetry, not a delivery guarantee for data that was never ingested.
