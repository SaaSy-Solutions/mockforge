# MockForge Bench

Load and performance testing for MockForge that leverages OpenAPI specifications to generate realistic traffic patterns against real services.

## Overview

MockForge Bench transforms MockForge from a mock server into a comprehensive load testing tool. By parsing OpenAPI specifications, it can automatically generate and execute load tests against real API endpoints using k6.

## Features

- **Spec-driven testing**: Automatically generate load tests from OpenAPI/Swagger specs
- **Multiple load scenarios**: Support for constant, ramp-up, spike, stress, and soak tests
- **Operation filtering**: Test specific endpoints or all operations
- **Realistic data**: Generate request data from spec schemas or examples
- **Customizable thresholds**: Set response time and error rate thresholds
- **Detailed reporting**: Per-operation metrics and summary reports
- **Script generation**: Generate k6 scripts for manual execution or CI/CD integration

## Prerequisites

- **k6**: The load testing tool must be installed
  ```bash
  # macOS
  brew install k6

  # Linux
  wget https://github.com/grafana/k6/releases/download/v0.48.0/k6-v0.48.0-linux-amd64.tar.gz
  tar -xzf k6-v0.48.0-linux-amd64.tar.gz
  sudo mv k6-v0.48.0-linux-amd64/k6 /usr/local/bin/

  # Windows
  choco install k6
  ```

## Usage

### Basic Load Test

```bash
mockforge bench --spec api.yaml --target https://api.example.com
```

### Advanced Options

```bash
# Run a 5-minute load test with 100 virtual users
mockforge bench \
  --spec api.yaml \
  --target https://staging.api.com \
  --duration 5m \
  --vus 100 \
  --scenario ramp-up

# Test specific operations only
mockforge bench \
  --spec api.yaml \
  --target https://api.com \
  --operations "GET /users,POST /users" \
  --auth "Bearer token123"

# Generate k6 script without running
mockforge bench \
  --spec api.yaml \
  --target https://api.com \
  --generate-only \
  --script-output bench.js

# Custom thresholds and headers
mockforge bench \
  --spec api.yaml \
  --target https://api.com \
  --threshold-percentile p99 \
  --threshold-ms 1000 \
  --max-error-rate 0.01 \
  --headers "X-API-Key:abc123,X-Client-ID:client456"
```

## Load Scenarios

### Constant Load
Maintains a steady number of virtual users throughout the test.

```bash
mockforge bench --spec api.yaml --target https://api.com --scenario constant --vus 50
```

### Ramp-up (Default)
Gradually increases load to the target number of virtual users.

```bash
mockforge bench --spec api.yaml --target https://api.com --scenario ramp-up --vus 100
```

### Spike Test
Simulates a sudden spike in load to test system resilience.

```bash
mockforge bench --spec api.yaml --target https://api.com --scenario spike --vus 200
```

### Stress Test
Continuously increases load to find the breaking point.

```bash
mockforge bench --spec api.yaml --target https://api.com --scenario stress --vus 500
```

### Soak Test
Maintains sustained load over an extended period to detect memory leaks and degradation.

```bash
mockforge bench --spec api.yaml --target https://api.com --scenario soak --duration 1h --vus 50
```

## Output

Results are saved to the output directory (default: `bench-results/`):

```
bench-results/
├── k6-script.js          # Generated k6 script
├── summary.json          # Test results in JSON format
└── detailed-results.json # Detailed per-operation metrics
```

### Terminal Output

```
MockForge Bench - Load Testing Mode
────────────────────────────────────────────────────────────

Specification: api.yaml
Target: https://api.example.com
Operations: 5 endpoints
Scenario: ramp-up
Duration: 120s

→ Loading OpenAPI specification...
✓ Specification loaded
→ Extracting API operations...
✓ Found 5 operations
→ Generating request templates...
✓ Request templates generated
→ Generating k6 load test script...
✓ k6 script generated
✓ Script written to: bench-results/k6-script.js
→ Executing load test...

Load Test Complete! ✓

Summary:
  Total Requests:       15,680
  Successful:           15,612 (99.57%)
  Failed:               68 (0.43%)

Response Times:
  Avg:                  156ms
  p95:                  356ms
  p99:                  523ms

  Throughput:           130.7 req/s

Results saved to: bench-results/
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: API Load Test

on:
  pull_request:
  workflow_dispatch:

jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install k6
        run: |
          wget https://github.com/grafana/k6/releases/download/v0.48.0/k6-v0.48.0-linux-amd64.tar.gz
          tar -xzf k6-v0.48.0-linux-amd64.tar.gz
          sudo mv k6-v0.48.0-linux-amd64/k6 /usr/local/bin/

      - name: Install MockForge
        run: cargo install --path .

      - name: Run load test
        run: |
          mockforge bench \
            --spec api/openapi.yaml \
            --target ${{ secrets.STAGING_API_URL }} \
            --duration 2m \
            --vus 50 \
            --output bench-results/ \
            --auth "Bearer ${{ secrets.API_TOKEN }}"

      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: load-test-results
          path: bench-results/
```

## API

### BenchCommand

The main command structure:

```rust
use mockforge_bench::BenchCommand;
use std::path::PathBuf;

let cmd = BenchCommand {
    spec: PathBuf::from("api.yaml"),
    target: "https://api.example.com".to_string(),
    duration: "5m".to_string(),
    vus: 100,
    scenario: "ramp-up".to_string(),
    operations: None,
    auth: Some("Bearer token123".to_string()),
    headers: None,
    output: PathBuf::from("bench-results"),
    generate_only: false,
    script_output: None,
    threshold_percentile: "p(95)".to_string(),
    threshold_ms: 500,
    max_error_rate: 0.05,
    verbose: false,
};

cmd.execute().await?;
```

## Architecture

```
┌─────────────────────────────────────────┐
│  OpenAPI Spec                           │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  SpecParser                             │
│  - Parse OpenAPI spec                   │
│  - Extract operations                   │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  RequestGenerator                       │
│  - Generate request templates           │
│  - Extract parameters and bodies        │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  K6ScriptGenerator                      │
│  - Generate k6 JavaScript               │
│  - Configure scenarios and thresholds   │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  K6Executor                             │
│  - Execute k6 with generated script     │
│  - Stream output and progress           │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Reporter                               │
│  - Parse results                        │
│  - Display summary                      │
└─────────────────────────────────────────┘
```

## Development

### Building

```bash
cargo build --release -p mockforge-bench
```

### Testing

```bash
cargo test -p mockforge-bench
```

### Adding New Load Scenarios

Extend the `LoadScenario` enum in `scenarios.rs`:

```rust
pub enum LoadScenario {
    Constant,
    RampUp,
    Spike,
    Stress,
    Soak,
    YourNewScenario,  // Add here
}

impl LoadScenario {
    pub fn generate_stages(&self, duration_secs: u64, max_vus: u32) -> Vec<Stage> {
        match self {
            // ... existing scenarios
            Self::YourNewScenario => {
                // Define your custom load pattern
                vec![/* your stages */]
            }
        }
    }
}
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

## License

MIT OR Apache-2.0
