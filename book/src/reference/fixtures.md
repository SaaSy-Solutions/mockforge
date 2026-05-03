# Fixtures and Smoke Testing

MockForge supports recording and replaying HTTP requests and responses as fixtures, which can be used for smoke testing your APIs.

## Recording Fixtures

Recording is enabled via the `--record` CLI flag or `core.fixtures.record:
true` in YAML. By default all HTTP requests are recorded. To record only
GET requests, set `core.fixtures.record_get_only: true`. Fixtures are
saved in `MOCKFORGE_FIXTURES_DIR` (default `./fixtures`).

## Replay Fixtures

Replay is enabled via the `--replay` CLI flag or `core.fixtures.replay:
true` in YAML. When replay is enabled, MockForge serves recorded responses
for matching requests instead of generating new ones.

## Ready-to-Run Fixtures

Fixtures can be marked as "ready-to-run" for smoke testing by adding a metadata field `smoke_test` with the value `true`. These fixtures will be listed in the smoke test endpoints.

Example fixture with smoke test metadata:

```json
{
  "fingerprint": {
    "method": "GET",
    "path": "/api/users",
    "query_params": {},
    "headers": {}
  },
  "timestamp": "2024-01-15T10:30:00Z",
  "status_code": 200,
  "response_headers": {
    "content-type": "application/json"
  },
  "response_body": "{\"users\": []}",
  "metadata": {
    "smoke_test": "true",
    "name": "Get Users Endpoint"
  }
}
```

## Smoke Testing

MockForge provides endpoints to list and run smoke tests:

- `GET /__mockforge/smoke` - List available smoke test endpoints
- `GET /__mockforge/smoke/run` - Run all smoke tests

These endpoints are also available in the Admin UI under the "Smoke Tests" tab.

## Admin UI Integration

The Admin UI provides a graphical interface for managing fixtures and running smoke tests:

1. View all recorded fixtures in the "Fixtures" tab
2. Mark fixtures as ready-to-run for smoke testing
3. Run smoke tests with a single click
4. View smoke test results and status

## Configuration

The following environment variables control fixture and smoke test behavior:

### Core Settings (env vars)
- `MOCKFORGE_FIXTURES_DIR` - Directory where fixtures are stored (default: `./fixtures`)
- `MOCKFORGE_LATENCY_ENABLED` - Inject latency on responses (default: `false`)
- `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND` - Expand `{{...}}` templates in responses (default: `false`)
- `MOCKFORGE_REQUEST_VALIDATION` - Validation level for incoming requests (`enforce` | `warn` | `off`)
- `MOCKFORGE_RESPONSE_VALIDATION` - Validate generated responses against the spec (`true` | `false`)

### Record / replay toggles (CLI / YAML, not env)

`record_enabled`, `replay_enabled`, and `record_get_only` are configured
under `core.fixtures.*` in YAML or via the `--record` / `--replay` CLI
flags — they don't have env-var equivalents.

### Configuration File Support
You can also configure fixtures through YAML:

```yaml
# In your configuration file
core:
  fixtures:
    dir: "./fixtures"
    record_enabled: false
    replay_enabled: false
    record_get_only: false
```
