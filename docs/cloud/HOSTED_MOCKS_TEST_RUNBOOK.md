# Hosted Mocks Live Test Runbook

End-to-end manual verification checklist for the hosted-mocks surface.
Audience: anyone validating the post-merge claims for PRs #237–#253 on a
staging or production deployment. Every item maps back to a PR's "live
test" line that was left unchecked at merge.

If you're an operator setting up the registry server, see
[HOSTED_MOCKS_OPERATOR.md](./HOSTED_MOCKS_OPERATOR.md) instead.

---

## Setup

Run these once per session.

### 1. Deploy a hosted mock

```bash
# Through the cloud admin UI — easier:
#   https://app.mockforge.dev/hosted-mocks → Create new deployment
#   - name:           runbook-${USER}
#   - openapi_spec_url: https://petstore3.swagger.io/api/v3/openapi.json
#   - upstream_url:   (leave empty for the reality-slider test below)
#   - enabled_protocols: HTTP, WebSocket, GraphQL, gRPC, Kafka, MQTT, AMQP, SMTP, TCP
#   - region:         iad (or closest)
#
# Wait for status to flip to "active" and health to "healthy" before
# proceeding. Typical wait is 60-120s.
```

Capture the deployment's public URL for the rest of the runbook:

```bash
DEPLOY_URL="https://runbook-<your-slug>.fly.dev"
DEPLOY_ID="<uuid from the URL or the response panel>"
TOKEN="<copy from the admin UI's API token settings>"
```

### 2. Auth helper

```bash
# All cloud-side endpoints require Bearer auth; tracked once here.
auth() { curl -sS -H "Authorization: Bearer $TOKEN" "$@"; }
```

### 3. Smoke

```bash
curl -sS "$DEPLOY_URL/__mockforge/api/health" | jq
auth https://api.mockforge.dev/api/v1/hosted-mocks/$DEPLOY_ID | jq '.status, .health_status'
# Expect: status="active", health_status="healthy"
```

If either of these is wrong, stop and check the deployment's events
tab in the admin UI.

---

## A. Protocol reachability (PRs #237, #226–#231, #243)

Verifies every protocol bundled into the cloud feature actually
reaches a real client.

- [ ] **HTTP** — `curl -sS "$DEPLOY_URL/health"` returns 200
- [ ] **WebSocket** — `wscat -c "$DEPLOY_URL/ws"` connects and stays
      open ≥10 s (port 3000 path; PR #219)
- [ ] **GraphQL** — `curl -sS -X POST "$DEPLOY_URL/graphql" -H "content-type: application/json" -d '{"query":"{__typename}"}'`
      returns a JSON result (port 3000 path; PR #220)
- [ ] **gRPC** — `grpcurl -insecure $DEPLOY_HOST:50051 list` returns
      the registered services (PR #226)
- [ ] **SMTP** — `swaks --to test@example.com --server $DEPLOY_HOST:2525`
      delivers and the message shows up under the deployment's `/smtp/mailbox`
      management endpoint (PR #227)
- [ ] **MQTT** — `mosquitto_pub -h $DEPLOY_HOST -p 1883 -t test -m "hello"`
      then `mosquitto_sub -h $DEPLOY_HOST -p 1883 -t test -C 1` echoes
      `hello` (PR #228)
- [ ] **Kafka — bootstrap** — `kcat -L -b $DEPLOY_HOST:9092` lists
      brokers and the broker hostname is `<app>.fly.dev`, **not**
      `0.0.0.0` (PR #243 wire-format fix)
- [ ] **Kafka — produce/consume** — `kcat -P -b $DEPLOY_HOST:9092 -t demo`
      and `kcat -C -b $DEPLOY_HOST:9092 -t demo -e -q` round-trips a
      message (PR #231)
- [ ] **Kafka — multi-partition + consumer groups** — produce to a
      topic with ≥2 partitions and confirm group offsets persist across
      a rebalance (PR #231 acceptance criteria)
- [ ] **AMQP** — `rabbitmqadmin --host=$DEPLOY_HOST --port=5672 list queues`
      returns a non-error response (PR #229)
- [ ] **TCP** — `nc -v $DEPLOY_HOST 9999` connects (PR #230)

---

## B. Observability (PRs #232–#234, #237, #238)

### Runtime logs

- [ ] Send 5 requests to `$DEPLOY_URL/anything`
- [ ] Hit the admin UI's **Logs** tab on the deployment-detail dialog
- [ ] Confirm structured request log entries appear within 5 s
      (server-side log shipper, PR #232)

### Captures

- [ ] Send a `POST $DEPLOY_URL/echo` with a JSON body
- [ ] Hit the **Captures** tab — entry appears with method/path/status
      and clickable detail
- [ ] Click **Replay** on the capture detail dialog → re-runs against
      the deployment and shows the response (PR #236)
- [ ] **Export HAR** → file downloads, opens in a browser, contains
      that capture
- [ ] **Export JSONL** → file downloads (PR #239); run
      `mockforge-cli replay captures.jsonl` locally and confirm the
      mock serves the captured exchange
- [ ] **Persistence across restart** (PR #234 part 2 / PR #240): hit
      the Fly machine's restart endpoint, wait for "active" again,
      reopen Captures — entries from before the restart still listed
- [ ] **Cloud-Postgres serving** (PR #242): once cloud-sync has at
      least one row, the proxy is bypassed — verify by checking the
      response includes captures from a previous deploy (different slug)

### Traces

- [ ] Set `MOCKFORGE_OTLP_INGEST_ENDPOINT` on the registry server (so
      the orchestrator wires the deployment env)
- [ ] Send 10+ requests through the deployment
- [ ] **Traces** tab on the deployment-detail dialog lists recent traces
      within 5 s (PR #237 storage, PR #238 UI)
- [ ] Click a trace → span waterfall renders, depth-indented by
      `parent_span_id`, error spans painted red
- [ ] Verify retention: pick a Free-tier org, wait 25 h after a trace,
      confirm the row was pruned (PR #241; takes a 24 h+ wait — usually
      simulated by setting `MOCKFORGE_TRACE_RETENTION_DAYS_FREE=0`)

### Metrics

- [ ] Configure `FLY_PROMETHEUS_URL` + `FLY_PROMETHEUS_TOKEN` on the
      registry server
- [ ] **Metrics** tab shows non-zero requests, p50, p95 latency
      reflecting the load you generated (PR #221)

---

## C. Behavioral controls (PRs #244, #246–#248, #253)

### Reality slider (mock ↔ proxy)

PR #244. Requires re-creating the deployment with `upstream_url` set:

```bash
# Through admin UI: edit deployment → set upstream_url to a real API
# or a second mockforge instance you control. e.g. https://httpbin.org
```

Then drive the slider on the deployment's workspace state via the
consistency engine:

```bash
# Set ratio to 0 — always-mock path
curl -sS -X POST "$DEPLOY_URL/api/v1/consistency/reality-ratio?workspace=default" \
  -H "content-type: application/json" -d '{"ratio": 0.0}'
curl -sS "$DEPLOY_URL/anything" -I | grep -i x-mockforge-source
# Expect: header absent (mock path)

# Set ratio to 1 — always-proxy
curl -sS -X POST "$DEPLOY_URL/api/v1/consistency/reality-ratio?workspace=default" \
  -H "content-type: application/json" -d '{"ratio": 1.0}'
curl -sS "$DEPLOY_URL/anything" -I | grep -i x-mockforge-source
# Expect: X-MockForge-Source: upstream

# 0.5 — coin-flip
curl -sS -X POST "$DEPLOY_URL/api/v1/consistency/reality-ratio?workspace=default" \
  -H "content-type: application/json" -d '{"ratio": 0.5}'
for i in $(seq 1 100); do curl -sS "$DEPLOY_URL/anything" -I 2>/dev/null \
    | grep -i x-mockforge-source; done | sort | uniq -c
# Expect: ~50/50 split between "upstream" and (no header)
```

- [ ] All three ratios behave as expected

### Route-scoped chaos (runtime)

PR #246. POST a fault rule:

```bash
curl -sS -X POST "$DEPLOY_URL/__mockforge/api/route-chaos/route" \
  -H "content-type: application/json" -d '{
    "method": "GET",
    "path": "/health",
    "request": null,
    "response": {"status": 200, "headers": {}, "body": null},
    "fault_injection": {"enabled": true, "probability": 1.0,
      "fault_types": [{"type": "http_error", "status_code": 503,
                       "message": "chaos engineered failure"}]},
    "latency": null
  }'

curl -sS "$DEPLOY_URL/health" -I | head -1
# Expect: HTTP/1.1 503

# Cleanup
curl -sS -X DELETE "$DEPLOY_URL/__mockforge/api/route-chaos/route?method=GET&path=/health"
```

- [ ] 503 returned with `X-MockForge-Source: route-chaos-runtime`
- [ ] After delete, `/health` returns 200 again

### Network profile

PR #247. Switch profiles at runtime:

```bash
curl -sS "$DEPLOY_URL/__mockforge/api/network-profiles" | jq '.profiles[].name'
# Lists available profiles

curl -sS -X POST "$DEPLOY_URL/__mockforge/api/network-profiles/mobile_3g/activate"
time curl -sS "$DEPLOY_URL/health" >/dev/null
# Expect: noticeably slower than baseline

curl -sS -X POST "$DEPLOY_URL/__mockforge/api/network-profiles/deactivate"
```

- [ ] Latency injection observable; deactivate restores baseline

### Named scenarios

PR #248. Requires a scenario installed locally on the deployment.
Without one the API lists empty, which is itself a valid pass.

- [ ] `curl -sS $DEPLOY_URL/__mockforge/api/scenarios` returns 200 with
      a `scenarios` array (empty is OK)
- [ ] If a scenario is installed: POST `…/scenarios/<name>/activate`,
      then GET `…/scenarios/active` echoes the name
- [ ] Subsequent requests carry `X-MockForge-Scenario: <name>` response
      header

### Time travel

PR #253. The endpoints used to be admin-only on port 9080; now reachable
on port 3000:

```bash
# Anchor virtual clock at New Year 2030
curl -sS -X POST "$DEPLOY_URL/__mockforge/time-travel/enable" \
  -H "content-type: application/json" \
  -d '{"time": "2030-01-01T00:00:00Z"}'

curl -sS "$DEPLOY_URL/__mockforge/time-travel/status" | jq .now
# Expect: time near 2030-01-01

# Advance by a week
curl -sS -X POST "$DEPLOY_URL/__mockforge/time-travel/advance" \
  -H "content-type: application/json" -d '{"duration": "1week"}'
curl -sS "$DEPLOY_URL/__mockforge/time-travel/status" | jq .now
# Expect: ~2030-01-08

# Reset
curl -sS -X POST "$DEPLOY_URL/__mockforge/time-travel/reset"
```

- [ ] `enable` succeeds and clock advances
- [ ] `advance` accepts `1week`, `2h`, `30m`, `250ms`
- [ ] `reset` returns clock to real time

---

## D. Test infrastructure (PRs #245, #250, #251)

### Request chains

PR #245. Mounted at `/__mockforge/chains`:

```bash
curl -sS "$DEPLOY_URL/__mockforge/chains"
# Expect: JSON list (empty initially is fine)

# Create a chain via the UI's ChainsPage and confirm it round-trips
```

- [ ] ChainsPage in admin UI lists, edits, and executes chains
      successfully against the deployment

### Fixtures upload

PR #250. New API on the main HTTP port:

```bash
# Create
curl -sS -X POST "$DEPLOY_URL/__mockforge/fixtures" \
  -H "content-type: application/json" -d '{
    "name": "smoke-test",
    "method": "GET",
    "path": "/users/42",
    "content": {"id": 42, "name": "Test User"}
  }'

# List
curl -sS "$DEPLOY_URL/__mockforge/fixtures" | jq '.[].id'
# Expect: includes "smoke-test"

# Download
curl -sS "$DEPLOY_URL/__mockforge/fixtures/smoke-test/download" | jq

# Delete
curl -sS -X DELETE "$DEPLOY_URL/__mockforge/fixtures/smoke-test"
```

- [ ] Round-trip works; UI Fixtures page shows the same data

### State machines tab

PR #251. Cloud-side proxy + deployment-detail tab:

- [ ] Define a state machine on the deployment via
      `POST $DEPLOY_URL/__mockforge/api/state-machines`
- [ ] Open the deployment-detail dialog → **State Machines** tab
- [ ] Definition appears in the top table
- [ ] Drive a state transition; the bottom table populates with the
      live instance

---

## E. Contract correctness (PRs #245, #249)

### Schema validation default-on

PR #245. Send a request whose response will violate the OpenAPI schema:

```bash
# Deploy with a spec that says GET /users returns an array; have the
# mock return a single object instead. Easiest: temporarily edit a
# fixture to be malformed.

curl -sS "$DEPLOY_URL/users" -i
# Expect: 5xx with a body explaining the validation failure, or a
# warn-level entry in the runtime logs tab depending on
# MOCKFORGE_RESPONSE_VALIDATION_MODE.
```

- [ ] Validation runs out of the box (no `MOCKFORGE_RESPONSE_VALIDATION=1`
      override needed — Dockerfile sets it)

### Contract diff retrieval

PR #249. Use the capture middleware + analyser:

```bash
# Send a few requests through the deployment first

curl -sS "$DEPLOY_URL/__mockforge/api/contract-diff/captures?limit=10" | jq '.count'
# Expect: > 0

# Analyse the most recent
curl -sS -X POST "$DEPLOY_URL/__mockforge/api/contract-diff/analyze?limit=5" | jq '.results[].ok'
# Expect: list of true/false; non-empty
```

- [ ] Both endpoints respond, results contain expected shape

---

## F. Verification assertions

PR pre-existing. Mounted at `/api/verification/*`:

```bash
# Send some requests first, then assert
curl -sS -X POST "$DEPLOY_URL/api/verification/at-least" \
  -H "content-type: application/json" \
  -d '{"path": "/health", "method": "GET", "min_count": 1}'
# Expect: { "passed": true }
```

- [ ] verify / count / sequence / never / at-least all return 200

---

## G. Dynamic responses (PRs #245, #252)

### Handlebars templating default-on

PR #245. The Dockerfile sets `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true`.

- [ ] Send a request whose mocked response uses `{{uuid}}` or `{{now}}`
- [ ] Confirm the template was expanded (not echoed literally) — for
      example, `id` is a real UUID, not the string `{{uuid}}`

### MockAI standalone

PR #252. Endpoint surfaces 503 when no API key is configured:

```bash
# Without API key
curl -sS -i "$DEPLOY_URL/__mockforge/api/mockai/status" | head
curl -sS -i -X POST "$DEPLOY_URL/__mockforge/api/mockai/generate" \
  -H "content-type: application/json" -d '{"path": "/users/42"}' | head
# Expect: status 503, error: "mockai_unavailable"
```

- [ ] Returns 503 when not configured (the contract documented in PR)
- [ ] With `OPENAI_API_KEY` set on the deployment, `/generate` returns a
      synthesized JSON response and `/status` reports `available: true`

---

## H. Persistence + retention

- [ ] Wait 6 h, confirm `runtime_request_logs` retention worker has
      pruned at least one row for a Free-tier org
      (`SELECT count(*) FROM runtime_request_logs WHERE created_at < now() - interval '24 hours'`)
- [ ] Same check against `runtime_captures` and `runtime_traces`
      (PR #241)

---

## Result tracking

When you finish a section, paste a one-line summary into the relevant PR
as a comment:

```
✅ Live-tested on staging deployment <slug> at <date>: all checks pass.
```

If anything fails, file an issue and link the PR — the runbook items
were the unchecked acceptance criteria, so a failure means we shipped a
gap rather than closed it.
