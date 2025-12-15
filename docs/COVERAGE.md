# MockForge Coverage

## Overview

This document covers two types of coverage in MockForge:

1. **Code Coverage**: Test coverage of Rust source code across all crates
2. **API Coverage**: Endpoint coverage tracking for OpenAPI specifications

---

## Code Coverage

### Overview

Code coverage tracks how much of the MockForge source code is exercised by tests. The project maintains an 80% coverage threshold across all crates.

### Current Status

**Last Updated**: Run `./scripts/coverage-baseline.sh` to generate latest report

**Coverage Summary**:
- See `coverage/summary.json` for detailed per-crate coverage
- See `coverage/summary.txt` for human-readable summary
- See `coverage/summary.csv` for spreadsheet-compatible data

### Coverage Thresholds

Per-crate thresholds are defined in `coverage.toml`:

- **Default**: 80%
- **High-Priority Crates** (core, http, cli, sdk): 85%
- **Protocol Crates**: 75%
- **Infrastructure Crates**: 70-75%

### Generating Coverage Reports

```bash
# Generate coverage baseline for all crates
./scripts/coverage-baseline.sh

# Generate with HTML reports
./scripts/coverage-baseline.sh --html

# Generate in parallel (faster)
./scripts/coverage-baseline.sh --parallel

# Generate specific format
./scripts/coverage-baseline.sh --format json
./scripts/coverage-baseline.sh --format csv
./scripts/coverage-baseline.sh --format text
```

### Coverage Reports Location

All coverage reports are stored in the `coverage/` directory:

```
coverage/
├── summary.json          # JSON summary with all crates
├── summary.csv           # CSV summary for spreadsheet import
├── summary.txt           # Human-readable text summary
├── lcov.info            # LCOV format for tools
└── crates/              # Per-crate reports
    ├── mockforge-core/
    │   ├── coverage.json
    │   ├── lcov.info
    │   └── index.html   # HTML report (if --html used)
    └── ...
```

### Coverage Dashboard

View the latest coverage summary:

```bash
cat coverage/summary.txt
```

Or view in JSON format:

```bash
cat coverage/summary.json | jq .
```

### Per-Crate Coverage

View coverage for a specific crate:

```bash
# JSON format
cat coverage/crates/mockforge-core/coverage.json | jq .

# HTML format (if generated)
open coverage/crates/mockforge-core/index.html
```

### CI Integration

Coverage is automatically generated in CI for all pull requests:

- Coverage reports are uploaded as artifacts
- Coverage summary is posted as a PR comment
- Coverage trends are tracked over time

### Improving Coverage

1. **Identify Gaps**: Review coverage reports to find untested code
2. **Prioritize**: Focus on high-impact, low-coverage areas first
3. **Add Tests**: Write tests for uncovered code paths
4. **Verify**: Re-run coverage to confirm improvements

See [TESTING_STANDARDS.md](TESTING_STANDARDS.md) for testing guidelines.

---

## API Endpoint Coverage

### Overview

Mock Coverage is a feature that tracks which API endpoints from your OpenAPI specification have been exercised during testing. Similar to code coverage in testing, mock coverage helps you identify untested parts of your API contracts.

## Key Features

- **Route Tracking**: Automatically tracks which routes have been called
- **Hit Counts**: Shows how many times each endpoint has been hit
- **Status Breakdown**: Displays distribution of HTTP status codes per endpoint
- **Latency Metrics**: Shows average response time for each endpoint
- **Method Coverage**: Breaks down coverage by HTTP method (GET, POST, etc.)
- **Real-time Updates**: Coverage updates automatically as requests are made
- **REST API**: Programmatic access via JSON API
- **Web UI**: Beautiful, interactive web interface

## How It Works

MockForge knows all routes defined in your OpenAPI spec and uses Prometheus metrics to track which routes have been called. The coverage calculation compares:

1. **All defined routes** (from OpenAPI spec)
2. **Routes that have been hit** (from metrics)

Coverage = (Covered Routes / Total Routes) × 100%

## API Endpoints

### GET `/__mockforge/coverage`

Returns complete coverage report in JSON format.

**Response:**
```json
{
  "total_routes": 8,
  "covered_routes": 3,
  "coverage_percentage": 37.5,
  "routes": [
    {
      "method": "GET",
      "path": "/users",
      "operation_id": "listUsers",
      "summary": "List all users",
      "covered": true,
      "hit_count": 15,
      "status_breakdown": {
        "200": 15
      },
      "avg_latency_seconds": 0.045
    },
    {
      "method": "POST",
      "path": "/users",
      "operation_id": "createUser",
      "summary": "Create a user",
      "covered": false,
      "hit_count": 0,
      "status_breakdown": {},
      "avg_latency_seconds": null
    }
  ],
  "method_coverage": {
    "GET": {
      "total": 4,
      "covered": 2,
      "percentage": 50.0
    },
    "POST": {
      "total": 2,
      "covered": 1,
      "percentage": 50.0
    }
  },
  "timestamp": "2025-10-09T12:00:00Z"
}
```

### Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `method` | string | Filter by HTTP method (e.g., "GET", "POST") |
| `path` | string | Filter by path pattern (e.g., "/users") |
| `uncovered_only` | boolean | Show only uncovered routes |

**Examples:**

```bash
# Get coverage for all GET endpoints
curl http://localhost:3000/__mockforge/coverage?method=GET

# Get only uncovered routes
curl http://localhost:3000/__mockforge/coverage?uncovered_only=true

# Filter by path pattern
curl http://localhost:3000/__mockforge/coverage?path=/users
```

### GET `/__mockforge/coverage.html`

Interactive web UI for visualizing coverage.

**Features:**
- Real-time coverage statistics
- Visual coverage percentage with color-coded bars
- Filterable route table
- Status indicators (covered/uncovered)
- Hit counts and latency metrics
- Auto-refresh every 5 seconds

## Web UI

The coverage UI provides an intuitive dashboard with:

### Statistics Panel
- **Overall Coverage**: Percentage of routes covered
- **Total Routes**: Total number of routes in your spec
- **Covered Routes**: Number of routes that have been called
- **Uncovered Routes**: Number of routes that haven't been called

### Filters
- Filter by HTTP method (GET, POST, PUT, DELETE, PATCH)
- Filter by path pattern
- Show only covered or uncovered routes

### Route Table
Each route shows:
- **Status**: Visual indicator (covered/uncovered)
- **Method**: HTTP method with color coding
- **Path**: Endpoint path
- **Hit Count**: Number of times called
- **Average Latency**: Average response time
- **Operation**: Operation ID or summary from OpenAPI spec

## Usage in Testing

### Example Test Scenario

```rust
use mockforge_http::{build_router, CoverageReport, calculate_coverage};

#[tokio::test]
async fn test_user_workflow() {
    // Start mock server
    let app = build_router(
        Some("./api-spec.json".to_string()),
        None,
        None
    ).await;

    // Start server in background
    let server = tokio::spawn(async {
        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
            .serve(app.into_make_service())
            .await
    });

    // Run your tests
    let client = reqwest::Client::new();

    // Test user creation
    client.post("http://localhost:3000/api/users")
        .json(&serde_json::json!({
            "name": "Alice",
            "email": "alice@example.com"
        }))
        .send()
        .await
        .unwrap();

    // Test user listing
    client.get("http://localhost:3000/api/users")
        .send()
        .await
        .unwrap();

    // Check coverage
    let coverage: CoverageReport = client
        .get("http://localhost:3000/__mockforge/coverage")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    println!("Coverage: {:.1}%", coverage.coverage_percentage);
    println!("Covered: {} / {}", coverage.covered_routes, coverage.total_routes);

    // Assert minimum coverage
    assert!(coverage.coverage_percentage >= 80.0,
        "Coverage too low: {:.1}%", coverage.coverage_percentage);
}
```

### CI/CD Integration

You can integrate coverage checks into your CI/CD pipeline:

```bash
#!/bin/bash
# coverage_check.sh

# Start MockForge server
mockforge --spec api-spec.json &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Run your test suite
pytest tests/

# Check coverage
COVERAGE=$(curl -s http://localhost:3000/__mockforge/coverage | jq -r '.coverage_percentage')

# Stop server
kill $SERVER_PID

# Check if coverage meets threshold
THRESHOLD=80.0
if (( $(echo "$COVERAGE < $THRESHOLD" | bc -l) )); then
    echo "❌ Coverage too low: ${COVERAGE}% (minimum: ${THRESHOLD}%)"
    exit 1
else
    echo "✅ Coverage: ${COVERAGE}%"
    exit 0
fi
```

## Path Normalization

MockForge normalizes paths to prevent metric cardinality explosion. Path parameters are replaced with `:id`:

| Original Path | Normalized Path |
|--------------|----------------|
| `/users/123` | `/users/:id` |
| `/users/550e8400-e29b-41d4-a716-446655440000` | `/users/:id` |
| `/users/{id}` | `/users/:id` |

This ensures that `/users/1`, `/users/2`, and `/users/999` all count toward the same route.

## Best Practices

### 1. Use Coverage to Find Gaps
Review uncovered routes to identify missing test cases:

```bash
curl http://localhost:3000/__mockforge/coverage?uncovered_only=true | jq '.routes[] | .path'
```

### 2. Track Coverage Over Time
Monitor coverage trends to ensure new endpoints are tested:

```bash
# Save coverage report
curl http://localhost:3000/__mockforge/coverage > coverage-$(date +%Y%m%d).json
```

### 3. Set Coverage Thresholds
Enforce minimum coverage in CI/CD (e.g., 80%):

```yaml
# .github/workflows/test.yml
- name: Check API Coverage
  run: |
    COVERAGE=$(curl -s http://localhost:3000/__mockforge/coverage | jq -r '.coverage_percentage')
    if (( $(echo "$COVERAGE < 80" | bc -l) )); then
      echo "Coverage too low: $COVERAGE%"
      exit 1
    fi
```

### 4. Review Method Coverage
Different HTTP methods may have different coverage:

```bash
curl http://localhost:3000/__mockforge/coverage | jq '.method_coverage'
```

### 5. Monitor Hit Counts
Unusually high hit counts may indicate inefficient tests:

```bash
curl http://localhost:3000/__mockforge/coverage | \
  jq '.routes[] | select(.hit_count > 100) | {path, hit_count}'
```

## Limitations

1. **No Reset Mechanism**: Coverage accumulates for the lifetime of the server. Restart the server to reset coverage.

2. **Metrics Dependency**: Coverage relies on Prometheus metrics. If metrics are disabled, coverage won't work.

3. **Path Normalization**: Dynamic path segments are normalized, so `/users/1` and `/users/2` count as the same route.

4. **No Historical Data**: Coverage is not persisted. Implement your own storage if you need historical tracking.

## Examples

See the [coverage_demo.rs](../examples/coverage_demo.rs) example for a complete demonstration.

## Related Features

- **Request Logging**: See all requests in `/__mockforge/logs`
- **Metrics**: Prometheus metrics at `/__mockforge/metrics`
- **Routes List**: All available routes at `/__mockforge/routes`

## Troubleshooting

### Coverage Shows 0%

**Cause**: No requests have been made yet.

**Solution**: Make some requests to your API endpoints.

### Coverage Doesn't Update

**Cause**: Path normalization mismatch.

**Solution**: Check that your request paths match the OpenAPI spec paths.

### Missing Routes in Coverage

**Cause**: Routes may not be in the OpenAPI spec.

**Solution**: Ensure all routes are defined in your OpenAPI specification.

### UI Not Loading

**Cause**: Static file not found.

**Solution**: Ensure `coverage.html` is in the correct location and the server has permission to read it.

## FAQ

**Q: Can I reset coverage without restarting the server?**

A: Not currently. Prometheus metrics are cumulative. Restart the server to reset coverage.

**Q: Does coverage work with AI-generated responses?**

A: Yes! Coverage tracks all routes regardless of how responses are generated.

**Q: Can I export coverage to other formats?**

A: The JSON API provides raw data. You can write scripts to convert it to any format you need.

**Q: Does coverage affect performance?**

A: Minimal impact. Coverage uses existing Prometheus metrics with no additional overhead per request.

**Q: Can I use coverage with WebSocket or gRPC?**

A: Currently only HTTP REST endpoints are supported.
