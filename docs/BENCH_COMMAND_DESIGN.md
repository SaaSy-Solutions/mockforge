# Load and Performance Testing Mode Design

## Overview

This document outlines the design for a new `mockforge bench` command that transforms MockForge from a mock server into a load testing tool for real services. This leverages MockForge's existing request generation capabilities from OpenAPI specs.

## Motivation

MockForge already:
- Parses OpenAPI specifications
- Generates k6 test scripts from recorded requests
- Understands request/response patterns
- Has comprehensive test data generation

By adding a bench mode, MockForge becomes a complete API testing toolkit that can both mock and test real services.

## Architecture

### 1. Command Structure

```rust
/// Load test a real service using an API specification
///
/// Examples:
///   mockforge bench --spec api.yaml --target https://api.example.com
///   mockforge bench --spec api.yaml --target https://staging.api.com --duration 5m --vus 100
///   mockforge bench --spec api.yaml --target https://api.com --scenario spike --output results/
///   mockforge bench --spec api.yaml --target https://api.com --operations "GET /users,POST /users"
Bench {
    /// API specification file (OpenAPI/Swagger)
    #[arg(short, long)]
    spec: PathBuf,

    /// Target service URL
    #[arg(short, long)]
    target: String,

    /// Test duration (e.g., 30s, 5m, 1h)
    #[arg(short, long, default_value = "1m")]
    duration: String,

    /// Number of virtual users (concurrent connections)
    #[arg(long, default_value = "10")]
    vus: u32,

    /// Load test scenario (constant, ramp-up, spike, stress, soak)
    #[arg(long, default_value = "ramp-up")]
    scenario: String,

    /// Filter operations to test (comma-separated, e.g., "GET /users,POST /users")
    #[arg(long)]
    operations: Option<String>,

    /// Authentication header value (e.g., "Bearer token123")
    #[arg(long)]
    auth: Option<String>,

    /// Additional headers (format: "Key:Value,Key2:Value2")
    #[arg(long)]
    headers: Option<String>,

    /// Output directory for results
    #[arg(short, long, default_value = "bench-results")]
    output: PathBuf,

    /// Generate k6 script without running
    #[arg(long)]
    generate_only: bool,

    /// k6 script output path (when using --generate-only)
    #[arg(long)]
    script_output: Option<PathBuf>,

    /// Use recorded data for request bodies (requires --recorder-db)
    #[arg(long)]
    use_recorded_data: bool,

    /// Recorder database path for request data
    #[arg(long)]
    recorder_db: Option<PathBuf>,

    /// Request rate (requests per second) instead of VUs
    #[arg(long)]
    rate: Option<u32>,

    /// Fail fast on first error
    #[arg(long)]
    fail_fast: bool,

    /// Response time threshold percentile (p50, p75, p90, p95, p99)
    #[arg(long, default_value = "p95")]
    threshold_percentile: String,

    /// Response time threshold in milliseconds
    #[arg(long, default_value = "500")]
    threshold_ms: u64,

    /// Maximum acceptable error rate (0.0-1.0)
    #[arg(long, default_value = "0.05")]
    max_error_rate: f64,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}
```

### 2. Core Components

#### 2.1 Spec Parser & Request Generator

```
┌─────────────────────────────────────────────────────────────┐
│  OpenAPI Spec Parser                                        │
│  - Parse OpenAPI/Swagger specs                              │
│  - Extract operations (path, method, parameters, schemas)   │
│  - Filter operations based on user selection                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Request Template Generator                                 │
│  - Generate request templates for each operation            │
│  - Include path parameters, query params, headers, body     │
│  - Use example data from spec or generate synthetic data    │
│  - Support for authentication schemes                       │
└─────────────────────────────────────────────────────────────┘
```

**Leverages existing code:**
- `crates/mockforge-core/src/openapi/spec.rs` - OpenAPI parsing
- `crates/mockforge-core/src/openapi/route.rs` - Route extraction
- `crates/mockforge-data/` - Synthetic data generation

#### 2.2 k6 Script Generator

```
┌─────────────────────────────────────────────────────────────┐
│  k6 Script Generator                                        │
│  - Transform request templates into k6 test functions       │
│  - Configure load scenarios (ramp-up, spike, etc.)          │
│  - Add thresholds and checks                                │
│  - Include custom metrics (per-operation tracking)          │
└─────────────────────────────────────────────────────────────┘
```

**Leverages existing code:**
- `crates/mockforge-recorder/src/test_generation.rs` - k6 script generation
- Extend `TestFormat::K6` to support real endpoints

#### 2.3 Load Test Executor

```
┌─────────────────────────────────────────────────────────────┐
│  Load Test Executor                                         │
│  - Write k6 script to temporary file                        │
│  - Execute k6 with appropriate flags                        │
│  - Stream output to user (with progress indicators)         │
│  - Handle graceful shutdown on Ctrl+C                       │
└─────────────────────────────────────────────────────────────┘
```

#### 2.4 Results Analyzer & Reporter

```
┌─────────────────────────────────────────────────────────────┐
│  Results Analyzer                                           │
│  - Parse k6 JSON output                                     │
│  - Calculate key metrics (throughput, latency, errors)      │
│  - Compare against thresholds                               │
│  - Generate summary report                                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Reporter                                                   │
│  - Terminal output (colored, formatted)                     │
│  - JSON output for CI/CD integration                        │
│  - HTML report generation (optional)                        │
│  - Per-operation metrics breakdown                          │
└─────────────────────────────────────────────────────────────┘
```

### 3. Load Test Scenarios

#### Constant Load
```
VUs: ████████████████████ (constant)
     └─────────────────────► Time
```

#### Ramp-up (Default)
```
VUs:         ┌───────────
            /
          /
        /
      /
     └─────────────────────► Time
     0s    1m    2m    3m
```

#### Spike Test
```
VUs:         ▲
             │
             │
     ────────┴────────────► Time
```

#### Stress Test
```
VUs:                  ┌─────
                    /
                  /
                /
              /
            /
          /
     ────┴───────────────► Time
```

#### Soak Test
```
VUs: ┌─────────────────────┐
     │                     │
     │   (long duration)   │
     └─────────────────────┘
```

### 4. Request Generation Strategy

#### From OpenAPI Spec
1. **Path Parameters**: Extract from path template, generate realistic values
2. **Query Parameters**: Use schema or examples from spec
3. **Request Body**:
   - Use `example` field if present
   - Use `examples` collection if present
   - Generate from schema using MockForge's data generation
4. **Headers**:
   - Required headers from spec
   - User-provided headers (auth, custom)
5. **Authentication**: Support for Bearer, Basic, API Key

#### From Recorded Data (Optional)
When `--use-recorded-data` is enabled:
1. Query recorder database for matching operations
2. Extract real request bodies and parameters
3. Optionally parameterize/vary the data
4. Fall back to spec-based generation if no recordings exist

### 5. Implementation Plan

#### Phase 1: Core Functionality (MVP)
```rust
// New module: crates/mockforge-bench/
mockforge-bench/
├── src/
│   ├── lib.rs
│   ├── command.rs         // CLI command handling
│   ├── spec_parser.rs     // Parse OpenAPI and extract operations
│   ├── request_gen.rs     // Generate request templates
│   ├── k6_gen.rs          // Generate k6 scripts
│   ├── executor.rs        // Execute k6 and capture output
│   ├── analyzer.rs        // Parse and analyze results
│   ├── reporter.rs        // Format and display results
│   └── scenarios.rs       // Load test scenario definitions
├── Cargo.toml
└── README.md
```

**Dependencies:**
- `openapiv3` (already in use)
- `serde`, `serde_json` (already in use)
- `tokio` (already in use)
- `colored` (for terminal output)
- `indicatif` (for progress bars)

#### Phase 2: Advanced Features
- HTML report generation with charts
- Compare results across multiple runs
- Distributed load testing (multi-node)
- Custom JavaScript functions in k6 scripts
- WebSocket and gRPC benchmarking

#### Phase 3: Integration
- Integration with recorder for realistic test data
- AI-powered test scenario suggestions
- Automated regression detection
- Integration with CI/CD pipelines

### 6. Generated k6 Script Structure

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics per operation
const getUsersLatency = new Trend('get_users_latency');
const getUsersErrors = new Rate('get_users_errors');
const postUsersLatency = new Trend('post_users_latency');
const postUsersErrors = new Rate('post_users_errors');

// Load scenario configuration
export const options = {
  stages: [
    { duration: '30s', target: 5 },
    { duration: '1m', target: 10 },
    { duration: '30s', target: 0 },
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500'],
    'http_req_failed': ['rate<0.05'],
  },
};

const BASE_URL = 'https://api.example.com';

export default function () {
  // Operation: GET /users
  let res = http.get(`${BASE_URL}/users`, {
    headers: {
      'Authorization': 'Bearer token123',
      'Content-Type': 'application/json',
    },
  });

  let success = check(res, {
    'GET /users: status 200': (r) => r.status === 200,
    'GET /users: has body': (r) => r.body.length > 0,
  });

  getUsersLatency.add(res.timings.duration);
  getUsersErrors.add(!success);

  sleep(1);

  // Operation: POST /users
  const payload = JSON.stringify({
    name: 'Test User',
    email: 'test@example.com',
  });

  res = http.post(`${BASE_URL}/users`, payload, {
    headers: {
      'Authorization': 'Bearer token123',
      'Content-Type': 'application/json',
    },
  });

  success = check(res, {
    'POST /users: status 201': (r) => r.status === 201,
    'POST /users: created': (r) => r.body.includes('id'),
  });

  postUsersLatency.add(res.timings.duration);
  postUsersErrors.add(!success);

  sleep(1);
}

export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'bench-results/summary.json': JSON.stringify(data),
  };
}
```

### 7. Terminal Output

```
MockForge Bench - Load Testing Mode
────────────────────────────────────────────────────────────

Specification: api.yaml
Target: https://api.example.com
Operations: 5 endpoints
Duration: 2m
Virtual Users: 10 → 50 → 10 (ramp-up)

Generating k6 script... ✓
Starting load test...

Progress: [████████████████████          ] 75% | 1m 30s / 2m

Live Metrics:
  Requests:      12,450  (104.2 req/s)
  Errors:        42      (0.34%)
  Duration p95:  245ms
  Duration p99:  512ms

────────────────────────────────────────────────────────────

Load Test Complete! ✓

Summary:
  Total Requests:       15,680
  Successful:           15,612 (99.57%)
  Failed:               68 (0.43%)

  Response Times:
    Min:                12ms
    Avg:                156ms
    p50:                145ms
    p75:                198ms
    p90:                287ms
    p95:                356ms
    p99:                523ms
    Max:                1,234ms

  Throughput:           130.7 req/s
  Data Received:        45.2 MB
  Data Sent:            8.7 MB

Per-Operation Metrics:
  GET /users            4,523 req | 98ms p95 | 0.1% errors ✓
  POST /users           3,189 req | 234ms p95 | 0.8% errors ✓
  GET /users/{id}       5,234 req | 87ms p95 | 0.2% errors ✓
  PUT /users/{id}       1,456 req | 456ms p95 | 0.9% errors ✓
  DELETE /users/{id}    1,278 req | 123ms p95 | 0.3% errors ✓

Thresholds:
  ✓ p95 < 500ms         (356ms < 500ms)
  ✓ Error rate < 5%     (0.43% < 5%)

Results saved to: bench-results/run_20251009_143022/
  - summary.json
  - detailed-results.json
  - k6-script.js

────────────────────────────────────────────────────────────
```

### 8. Integration with Existing Features

#### Recorder Integration
```bash
# Record traffic from production
mockforge serve --recorder --spec api.yaml

# Replay recorded data against staging
mockforge bench --spec api.yaml --target https://staging.api.com \
  --use-recorded-data --recorder-db ./mockforge-recordings.db
```

#### AI Integration
```bash
# Generate optimal load test scenario
mockforge bench --spec api.yaml --target https://api.com \
  --ai-optimize --rag-provider openai
```

### 9. Use Cases

#### 1. Pre-deployment Performance Testing
```bash
mockforge bench --spec api.yaml --target https://staging.api.com \
  --scenario stress --duration 10m --vus 200
```

#### 2. API Regression Testing
```bash
mockforge bench --spec api.yaml --target https://api.com \
  --operations "GET /users,POST /users" --fail-fast
```

#### 3. Load Test Script Generation
```bash
mockforge bench --spec api.yaml --target https://api.com \
  --generate-only --script-output tests/load/api_bench.js
```

#### 4. CI/CD Integration
```yaml
# GitHub Actions
- name: Load test API
  run: |
    mockforge bench --spec api.yaml \
      --target ${{ secrets.API_URL }} \
      --duration 2m \
      --vus 50 \
      --output bench-results/ \
      --fail-fast

- name: Upload results
  uses: actions/upload-artifact@v3
  with:
    name: bench-results
    path: bench-results/
```

### 10. Benefits

1. **Unified Tooling**: One tool for mocking and testing
2. **Spec-Driven**: Leverage existing OpenAPI specs
3. **Realistic Testing**: Use recorded data for authentic requests
4. **Easy to Use**: Simple CLI with sensible defaults
5. **CI/CD Ready**: JSON output and exit codes for automation
6. **Flexible**: Support for custom scenarios and operations
7. **Insightful**: Per-operation metrics and detailed reports

### 11. Technical Considerations

#### k6 Dependency
- Require k6 to be installed on the system
- Check for k6 availability at runtime
- Provide clear installation instructions
- Consider bundling k6 in future releases

#### Performance
- Generate scripts efficiently (reuse OpenAPI parsing)
- Stream output instead of buffering
- Handle large specs gracefully

#### Error Handling
- Validate spec before generating tests
- Check target URL accessibility
- Handle k6 execution failures gracefully
- Provide actionable error messages

#### Security
- Sanitize user inputs (URLs, headers)
- Warn about sending credentials to targets
- Support environment variables for secrets
- Option to exclude sensitive operations

### 12. Future Enhancements

1. **Multi-protocol Support**: WebSocket and gRPC benchmarking
2. **Distributed Testing**: Coordinate multiple k6 instances
3. **Historical Tracking**: Store and compare results over time
4. **Smart Throttling**: Automatically adjust load based on target response
5. **Chaos Integration**: Combine bench with chaos testing
6. **Custom Metrics**: User-defined success criteria
7. **Report Sharing**: Upload results to cloud for team visibility

## Conclusion

The `mockforge bench` command transforms MockForge into a comprehensive API testing tool that can both mock and load test services. By leveraging existing capabilities (OpenAPI parsing, request generation, k6 script generation), this feature requires relatively modest implementation effort while providing significant value.

The design prioritizes:
- **Ease of use**: Simple CLI with sensible defaults
- **Flexibility**: Support for various scenarios and customization
- **Integration**: Works well with existing MockForge features
- **Production-ready**: Suitable for CI/CD pipelines

This positions MockForge as a complete API development and testing toolkit.
