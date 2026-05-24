# MockForge Grafana Dashboards

Dashboards-as-code for the MockForge observability stack. JSON files in `dashboards/` are checked into git as the source of truth; the production Grafana instance is provisioned from them.

## Why this exists

Before this directory, dashboards lived only in the running Grafana instance — there was no record of what panels existed, no review for changes, no easy way to roll back a broken dashboard, and no way to stand up a second environment (staging, preview) with parity.

Per the 2026-05-23 go-live readiness audit, this is one piece of observability blocker #3 (the others were [server-side Sentry](../crates/mockforge-observability/src/sentry_init.rs) and on-call provider integration).

## What's here

| Dashboard | UID | Covers |
|---|---|---|
| [Service Overview](dashboards/mockforge-service-overview.json) | `mockforge-service-overview` | Request rate, error rate, latency p50/p95/p99, in-flight requests — all by protocol and pillar. Top-10 paths by rate and latency. The default landing page for "is MockForge healthy right now?" |
| [Marketplace](dashboards/mockforge-marketplace.json) | `mockforge-marketplace` | Publish / download / search operations on the plugin/template/scenario marketplace. Operation rates, p95 durations, errors by code, total items. Spike in `errors_total` is the canary for the kind of bugs that drove PRs #621 / #624. |

## Metric sources

All dashboards reference real series exported at `/metrics` from:
- `mockforge-registry-server` (the multi-tenant SaaS registry on Fly)
- The per-tenant `mockforge` binary (hosted mocks, when scraped via Fly Managed Prometheus)

The full metrics catalogue is built up in `crates/mockforge-observability/src/prometheus/metrics.rs` (core) and `crates/mockforge-registry-server/src/metrics.rs` (marketplace). When you add a new dashboard, ground every PromQL expression in a metric that is actually `record_*`d somewhere — defined-but-never-recorded metrics will silently show empty panels.

## Importing into Grafana

### Via the UI (one-off)

1. Grafana → Dashboards → New → Import
2. Upload the JSON file
3. Pick your Prometheus datasource when prompted (the `${DS_PROMETHEUS}` template variable resolves from this).

### Via the HTTP API (CI / scripted)

```bash
# Set these once
export GRAFANA_URL="https://grafana.example.com"
export GRAFANA_TOKEN="…"  # service account token with Editor on the target folder

for f in grafana/dashboards/*.json; do
  jq '{dashboard: (.|del(.__inputs, .__requires)), overwrite: true, folderUid: ""}' "$f" \
    | curl -sS -X POST "$GRAFANA_URL/api/dashboards/db" \
        -H "Authorization: Bearer $GRAFANA_TOKEN" \
        -H "Content-Type: application/json" \
        --data @-
done
```

The `del(.__inputs, .__requires)` removes the import-only fields Grafana adds to exported dashboards; leaving them in tries to provision a datasource on every push.

### Via provisioning (recommended for prod)

In `grafana.ini` or the helm chart, mount this directory as a dashboard provider:

```yaml
apiVersion: 1
providers:
  - name: mockforge
    orgId: 1
    folder: MockForge
    type: file
    disableDeletion: false
    updateIntervalSeconds: 60
    allowUiUpdates: false  # source of truth is git
    options:
      path: /var/lib/grafana/dashboards/mockforge
```

`allowUiUpdates: false` is important — it prevents Grafana operators from editing in the UI without round-tripping through git. Without it, changes made in Grafana get overwritten on the next provisioning sync and people lose work silently.

## Updating a dashboard

1. Edit the JSON directly, or edit in Grafana and export.
2. If exported from Grafana: open the JSON and remove the `id`, the `__elements` map, and any `iteration` field at the top level. Set `version` back to 1 (Grafana increments it on save; the diff is noise).
3. Bump the `version` field if you want Grafana to detect a change on next provisioning sync.
4. Open a PR. CI should pick up dashboard JSON in any future linting we add.

## Conventions

- **UID** stable across edits. Don't rename — Grafana keeps URL aliases on UIDs, not titles.
- **Datasource variable name** is always `DS_PROMETHEUS`. Don't hard-code datasource UIDs.
- **PromQL** uses `clamp_min(..., 1)` on denominators of error-rate-style divisions so an idle period doesn't divide by zero.
- **Default time range** is `now-1h` to `now` for overview, `now-6h` to `now` for trend dashboards.
- **Refresh** is `30s` — matches Prometheus default scrape interval; a faster refresh wastes Prometheus.

## Not in scope here

- **Alert rules.** Tracked separately — alerts-as-code will land as a sibling `grafana/alerts/` directory using Grafana unified alerting YAML. The metrics being dashboarded here are the right starting point for alert thresholds (error rate > 1% sustained 5m, p99 > 1s sustained 5m, marketplace publish errors > 0 sustained 5m).
- **Per-tenant hosted-mock dashboards.** Fly Managed Prometheus aggregates per-app, so a per-tenant panel needs the `app` label which the orchestrator sets — easier to land after the orchestrator's `SENTRY_DSN` injection follow-up.
- **Business / SLO dashboards.** Several metrics (`mockforge_service_availability`, `mockforge_slo_compliance`, `mockforge_error_budget_remaining`) are defined in core but not yet `record_`d anywhere. Once they have data, they get their own dashboard.
