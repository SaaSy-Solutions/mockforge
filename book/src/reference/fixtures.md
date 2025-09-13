# Fixtures and Smoke Testing

MockForge supports recording and replaying HTTP requests and responses as fixtures, which can be used for smoke testing your APIs.

## Recording Fixtures

To record fixtures, enable recording by setting the environment variable:

```
MOCKFORGE_RECORD_ENABLED=true
```

By default, all HTTP requests will be recorded. To record only GET requests, set:

```
MOCKFORGE_RECORD_GET_ONLY=true
```

Fixtures are saved in the `fixtures` directory by default. You can change this location with:

```
MOCKFORGE_FIXTURES_DIR=/path/to/fixtures
```

## Replay Fixtures

To replay recorded fixtures, enable replay by setting the environment variable:

```
MOCKFORGE_REPLAY_ENABLED=true
```

When replay is enabled, MockForge will serve recorded responses for matching requests instead of generating new ones.

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

### Core Settings
- `MOCKFORGE_FIXTURES_DIR` - Directory where fixtures are stored (default: `./fixtures`)
- `MOCKFORGE_RECORD_ENABLED` - Enable recording of requests (default: `false`)
- `MOCKFORGE_REPLAY_ENABLED` - Enable replay of recorded requests (default: `false`)

### Recording Options
- `MOCKFORGE_RECORD_GET_ONLY` - Record only GET requests (default: `false`)
- `MOCKFORGE_LATENCY_ENABLED` - Include latency in recorded fixtures (default: `true`)
- `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND` - Expand templates when recording (default: `false`)

### Validation and Testing
- `MOCKFORGE_REQUEST_VALIDATION` - Validation level during recording (default: `enforce`)
- `MOCKFORGE_RESPONSE_VALIDATION` - Validate responses during replay (default: `false`)

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
