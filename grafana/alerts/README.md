# MockForge Grafana Alert Rules

Grafana unified alerting rules as code, sibling to the dashboards in [`../dashboards/`](../dashboards/). Closes #646.

## What's here

`mockforge-alerts.yaml` — five starter rules across two groups:

| Group | Rule | Severity | Trigger |
|---|---|---|---|
| `mockforge-service-health` | error rate elevated | warning | 5xx rate > 0.5% sustained 5m |
| `mockforge-service-health` | error rate spike | critical | 5xx rate > 1% sustained 5m |
| `mockforge-service-health` | p99 latency high | warning | p99 > 1s sustained 5m |
| `mockforge-service-health` | traffic silence canary | critical | zero requests in 10m (total-outage signal) |
| `mockforge-marketplace` | marketplace errors detected | warning | any marketplace errors sustained 5m |

The marketplace rule is sensitive on purpose — PR #624 closed the last cascade of marketplace bugs on 2026-05-23 and we want to know within minutes if a similar regression ships.

## Importing

### Via provisioning (recommended for prod)

Mount this file at `/etc/grafana/provisioning/alerting/mockforge-alerts.yaml`. Grafana reloads on its provisioning interval. UIDs are stable — do not rename them, alert history and silence rules key on UID.

For the helm chart, drop this in the `grafana.alerting` block:

```yaml
grafana:
  alerting:
    rules.yaml:
      apiVersion: 1
      # then paste the contents of mockforge-alerts.yaml under here,
      # OR mount this file via extraConfigmapMounts
```

### Via the HTTP API (one-off)

```bash
export GRAFANA_URL="https://grafana.example.com"
export GRAFANA_TOKEN="…"

# Provisioning API takes one group at a time
for group_name in mockforge-service-health mockforge-marketplace; do
  yq ".groups[] | select(.name == \"$group_name\")" grafana/alerts/mockforge-alerts.yaml \
    | yq -o=json \
    | curl -sS -X POST "$GRAFANA_URL/api/v1/provisioning/alert-rules" \
        -H "Authorization: Bearer $GRAFANA_TOKEN" \
        -H "Content-Type: application/json" \
        -H "X-Disable-Provenance: true" \
        --data @-
done
```

`X-Disable-Provenance: true` lets the alerts be edited in the UI later — drop it if you want to lock the rules to file-only.

## Contact point caveat

Until the on-call provider integration lands (separate work — PagerDuty / Opsgenie / Grafana OnCall), these rules emit to whatever default contact point is configured in your Grafana instance. Two options for now:

1. **Set Grafana's default contact point to a real email/Slack webhook** via Grafana → Alerting → Contact points. Fastest path to "alerts actually reach a human."
2. **Define a `contactpoints.yaml` and provision it alongside this file.** Better for git-as-source-of-truth but requires picking the provider first.

The rules use `team: mockforge` labels so a downstream routing policy can fan them out by severity once the provider is wired.

## Adding new rules

1. Pick a group (or create one) — group is the unit of evaluation interval and the natural folder for related rules.
2. Generate a stable UID. Format: `mockforge-<short-name>`. Don't reuse a UID after deleting a rule — Grafana will refuse.
3. Every PromQL expression must reference a metric that is actually `record_*`d. Defined-but-not-recorded metrics (e.g. `mockforge_error_rate`, `mockforge_service_availability`) silently never fire. Check `crates/mockforge-observability/src/prometheus/metrics.rs` and `crates/mockforge-registry-server/src/metrics.rs` before authoring.
4. The threshold expression goes in a separate `__expr__` ref — Grafana's evaluator runs it against the PromQL result. See the existing rules for the shape.
5. `for: 5m` is the minimum duration the condition has to hold before firing. Don't go below 2m — Prometheus scrape interval is 30s and you'll get false positives on single-sample blips.
6. `noDataState: OK` means "if no metrics arrived, treat as healthy." Use `Alerting` only for canaries where missing data IS the alert condition (see the traffic silence canary).

## Conventions

- **Labels** every rule sets `team: mockforge`. Marketplace rules also set `area: marketplace`. Use these in routing policies.
- **Annotations** are human-readable — `summary` is the one-liner shown in the inbox, `description` is the full context. Embed value templates via `{{ $values.C.Value | printf "%.2f%%" }}`.
- **Severity** is `warning` or `critical`. Three-tier (info/warning/critical) is overkill at our scale.
- **Datasource UID** is always `${DS_PROMETHEUS}` — replaced at provisioning time.

## Not in scope

- **Contact points + routing policy YAML.** Needs the on-call provider picked first. Filing as `grafana/alerts/contactpoints.yaml` is a separate PR.
- **Per-pillar alerts.** The metrics carry a `pillar` label (reality / contracts / devx / cloud / ai) — once we have customer signal on which pillar's failures hurt most, we can author pillar-scoped thresholds. For now top-level is enough.
- **Hosted-mock alerts.** Per-tenant alerting needs the orchestrator's `SENTRY_DSN` / app-label injection follow-up. Different domain, different rules.
