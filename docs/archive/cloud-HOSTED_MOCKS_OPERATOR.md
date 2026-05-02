# Hosted Mocks Operator Runbook

How to set up and maintain MockForge Cloud's hosted-mocks feature.
Audience: operators running the MockForge Cloud registry server (the Rust
binary in `crates/mockforge-registry-server`) on Fly.io or equivalent.

This document covers the **environment variable contract** for every
optional capability the registry server can enable on hosted-mock
deployments. If a capability's env vars aren't set, the server still
boots — that capability just degrades to a no-op or returns an empty
result. Every default is "off."

If you're a user trying to *use* hosted mocks, you want the
[`/hosted-mocks` page in the admin UI](https://app.mockforge.dev/hosted-mocks),
not this doc.

---

## How the system fits together

```
┌─────────────────────────┐    ┌──────────────────────────┐
│  MockForge Cloud Admin  │    │ Registry server          │
│  UI (Cloudflare Pages)  │◄──►│ (this binary, on Fly)    │
└─────────────────────────┘    └────────────┬─────────────┘
                                            │ Fly Machines API
                                            ▼
                               ┌──────────────────────────┐
                               │ Per-deployment Fly app   │
                               │ running mockforge-cli    │
                               │ image (`:cloud` build)   │
                               └──────────────────────────┘
```

For each user-created deployment:

1. The user calls `POST /api/v1/hosted-mocks` with a name + spec URL.
2. The orchestrator (`crates/mockforge-registry-server/src/deployment/orchestrator.rs`)
   creates a Fly app `mockforge-<orghash>-<slug>`, allocates IPs, sets
   env vars, and creates one or more services on a Fly machine.
3. The deployed `mockforge-cli serve --admin` reads the env vars to
   decide which protocols to enable, where to ship logs, where to send
   OTLP, etc.

---

## Required infrastructure

| Service | Purpose |
|---|---|
| Postgres | Primary store for orgs, deployments, runtime logs. The new `runtime_request_logs` migration ships in `crates/mockforge-registry-server/migrations/20250101000041_runtime_request_logs.sql`. |
| Fly.io account + org | Each hosted-mock deployment is a Fly app inside this org. |
| `JWT_SECRET` (env var) | Signs every JWT, including the deployment-scoped tokens minted for log ingest and OTLP. |
| Docker image | `ghcr.io/saasy-solutions/mockforge:latest` built with `--no-default-features --features cloud` (the standard `Dockerfile` in this repo). |

---

## Env var contract

### Required (registry server won't deploy hosted mocks without these)

| Variable | Purpose |
|---|---|
| `JWT_SECRET` | HMAC secret. Reused for both user JWTs and deployment-scoped ingest tokens. **Rotate periodically.** Set `JWT_SECRET_PREVIOUS` during rotation so existing tokens stay valid through the overlap window. |
| `FLYIO_API_TOKEN` | Fly.io API token with `deploy:write` scope on the target org. The orchestrator uses this to create apps + machines; the same token is reused by the runtime-logs proxy. |
| `FLYIO_ORG_SLUG` | Fly org slug (e.g., `mockforge-prod`). |
| `DATABASE_URL` | Standard Postgres connection string. |

### Optional — runtime logs (Fly Logs API, issue #224)

| Variable | Default | Purpose |
|---|---|---|
| `FLY_LOGS_URL` | `https://api.fly.io/api/v1` | Base URL for the Fly logs REST endpoint. Override if Fly moves it or if you stand up a regional log aggregator. |
| `FLY_LOGS_TIMEOUT_MS` | `5000` | Per-request timeout. |
| `FLY_LOGS_DEFAULT_LIMIT` | `200` | REST default page size when `?limit=` isn't given. |

If `FLYIO_API_TOKEN` is set (which it has to be for deploys to work),
the SSE log stream at `GET /api/v1/hosted-mocks/{id}/runtime-logs/stream`
is automatically active. No extra setup.

### Optional — runtime metrics (Fly Managed Prometheus, issue #221)

| Variable | Default | Purpose |
|---|---|---|
| `FLY_PROMETHEUS_URL` | unset | Fly Managed Prometheus base URL, typically `https://api.fly.io/prometheus/<org-slug>`. |
| `FLY_PROMETHEUS_TOKEN` | unset | Read-only Prometheus token. |
| `FLY_PROMETHEUS_APP_LABEL` | `app` | Prometheus label name used to scope queries to a specific Fly app. Override if Fly scrapes with a different label (e.g., `fly_app_name`). |
| `FLY_PROMETHEUS_TIMEOUT_MS` | `3000` | Per-PromQL-query timeout. |
| `FLY_PROMETHEUS_WINDOW_DAYS` | `30` | Window the metrics endpoint aggregates over — match this to the calendar period your billing/usage flow expects. |

When unset, the metrics endpoint falls back to the `deployment_metrics`
Postgres table — which is currently empty without an active writer.
You'll see zeros in the admin UI Metrics tab until Prometheus is wired up.

### Optional — structured request log shipping (issue #232)

| Variable | Default | Purpose |
|---|---|---|
| `MOCKFORGE_LOG_INGEST_BASE_URL` | unset | Public URL the deployed mockforge-cli POSTs request logs to, e.g., `https://api.mockforge.dev`. The orchestrator templates `<base>/api/v1/hosted-mocks/<id>/log-ingest` into the deployment's env. |

If unset, deployments run without structured request log capture and the
admin UI's "Requests" tab shows the empty state. The retention worker
runs regardless — it only does work when there's something to prune.

### Optional — runtime log retention overrides (issue #232)

The retention worker prunes `runtime_request_logs` every 6 hours by
plan tier:

| Plan | Default retention |
|---|---|
| Free | 24 hours |
| Pro | 7 days |
| Team | 30 days |

Override per tier via:

| Variable | Purpose |
|---|---|
| `MOCKFORGE_LOG_RETENTION_DAYS_FREE` | Override Free-tier window (in days). |
| `MOCKFORGE_LOG_RETENTION_DAYS_PRO` | Override Pro-tier window. |
| `MOCKFORGE_LOG_RETENTION_DAYS_TEAM` | Override Team-tier window. |

Useful for soak testing without a code change.

### Optional — OTLP tracing receiver (issue #233)

| Variable | Default | Purpose |
|---|---|---|
| `MOCKFORGE_OTLP_INGEST_ENDPOINT` | unset | Where deployments should send OTLP traces. The orchestrator passes this through as `MOCKFORGE_OTLP_ENDPOINT` on each Fly machine. |

The receiver at `POST /api/v1/hosted-mocks/{id}/otlp/v1/traces` is a
**scaffold today** — it validates the deployment-scoped token, counts
the spans it received, and logs at info level. Persistence and a
Traces tab in the admin UI are tracked as follow-ups on #233.

### Optional — custom domain for deployment URLs

| Variable | Purpose |
|---|---|
| `MOCKFORGE_MOCKS_DOMAIN` | If set, deployment URLs become `https://<slug>.<domain>` instead of `https://mockforge-<orghash>-<slug>.fly.dev`. Requires a wildcard cert on the registry server's Fly app for `*.<domain>`. |

---

## Plan-tier capability matrix

Reference for the protocol picker in the create-deployment dialog:

| Capability | Free | Pro | Team |
|---|---|---|---|
| HTTP (port 3000) | ✓ | ✓ | ✓ |
| WebSocket (HTTP upgrade on 3000) | ✓ | ✓ | ✓ |
| GraphQL (`/graphql` on 3000) | ✓ | ✓ | ✓ |
| gRPC (port 50051, h2/tls) | — | ✓ | ✓ |
| SMTP (port 2525) | — | — | ✓ |
| MQTT (port 1883) | — | — | ✓ |
| Kafka (port 9092) | — | — | ✓ |
| AMQP (port 5672) | — | — | ✓ |
| Raw TCP (port 9999) | — | — | ✓ |

Plan-gate enforcement is in `mockforge-registry-server::handlers::hosted_mocks::create_deployment`.
Users requesting a higher-tier protocol get a 400 with a friendly
"protocols require a higher plan" message.

FTP is intentionally out of scope (Fly's passive-port story is painful;
revisit if there's customer demand).

---

## Smoke test checklist

After updating any of these env vars, redeploy the registry server and
walk through:

- [ ] **Boot:** registry server logs show no startup errors. `GET /health`
  returns 200.
- [ ] **Deploy:** create a new hosted mock via the admin UI. Within ~30s
  the status flips from `pending` → `deploying` → `active`.
- [ ] **HTTP reachable:** `curl https://<deployment>.fly.dev/__mockforge/api/health`
  returns 200.
- [ ] **WS reachable:** `wscat -c wss://<deployment>.fly.dev/ws` opens.
- [ ] **GraphQL reachable:** `curl -X POST https://<deployment>.fly.dev/graphql
  -d '{"query":"{ __schema { types { name } } }"}'` returns introspection.
- [ ] **Events tab populates:** the modal's Events tab shows lifecycle
  entries.
- [ ] **Logs tab populates:** with `FLYIO_API_TOKEN` set, container
  stdout/stderr appears in the Logs tab within ~2s of any log line.
- [ ] **Metrics tab:** with Prometheus configured, send a few requests
  and watch counters tick up within ~60s.
- [ ] **Requests tab:** with `MOCKFORGE_LOG_INGEST_BASE_URL` configured,
  send traffic and verify rows appear within ~4s. Filter chips work.
- [ ] **Captures tab:** enable the recorder on the deployment
  (`POST /api/recorder/enable`), send traffic, verify the Captures tab
  populates. Click a row → detail dialog renders headers + body.
  "Export HAR" downloads a `.har` file.
- [ ] **Plan gating:** a Free-plan account creating a deployment with
  `enabled_protocols=["http","grpc"]` gets a 400 listing `Grpc`.
- [ ] **Retention:** verify the retention worker started by greping
  registry-server logs for `Runtime request logs retention worker started`.

---

## Troubleshooting

**"Fly Prometheus query failed; falling back to local counters"** in
registry-server logs → check `FLY_PROMETHEUS_TOKEN` is valid and the
`FLY_PROMETHEUS_APP_LABEL` matches what Fly is actually scraping. Some
Fly orgs scrape with `fly_app_name` rather than `app`.

**Captures tab shows empty even when recorder is enabled** → the
recorder API requires `observability.recorder.enabled = true` in the
deployment's `MOCKFORGE_CONFIG`. The flag is per-deployment, not a
global registry-server setting. Confirm by curling
`https://<deployment>.fly.dev/api/recorder/status`.

**Requests tab shows "MOCKFORGE_LOG_INGEST_BASE_URL configured" warning**
→ the env var is unset on the registry server. Set it in your Fly
secrets and redeploy the registry server. Existing hosted-mock
deployments need to be redeployed too — env vars are baked into the
machine config at deploy time.

**Kafka clients can't connect after bootstrap** → the
`mockforge-kafka` broker's `MetadataResponse` doesn't yet emit the
`advertised_host` value (tracked on #231). The cloud-side plumbing
sets `MOCKFORGE_KAFKA_ADVERTISED_HOST` correctly; the upstream broker
needs to consume it.

---

## Changelog

| PR | Summary |
|---|---|
| #236 | Initial operator-facing surface: protocol exposure, plan gating, real metrics, runtime log streaming, structured request log shipper + retention, recorder cloud proxy, OTLP scaffold. This document. |
