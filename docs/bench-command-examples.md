# MockForge Bench Command Examples

This document provides practical examples of using the `mockforge bench` command for various load testing scenarios.

## Table of Contents

1. [Basic Usage](#basic-usage)
2. [Load Test Scenarios](#load-test-scenarios)
3. [Authentication & Headers](#authentication--headers)
4. [Operation Filtering](#operation-filtering)
5. [Script Generation](#script-generation)
6. [CI/CD Integration](#cicd-integration)
7. [Advanced Configurations](#advanced-configurations)

## Basic Usage

### Quick Load Test

Test an API with default settings (1 minute, 10 VUs, ramp-up scenario):

```bash
mockforge bench --spec api.yaml --target https://api.example.com
```

### Specify Duration and Virtual Users

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --duration 5m \
  --vus 50
```

## Load Test Scenarios

### Constant Load Test

Maintain a steady 30 virtual users for 3 minutes:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --scenario constant \
  --duration 3m \
  --vus 30
```

### Ramp-up Test (Default)

Gradually increase load from 0 to 100 VUs:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://staging.api.com \
  --scenario ramp-up \
  --duration 10m \
  --vus 100
```

### Spike Test

Simulate a sudden traffic spike:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --scenario spike \
  --duration 5m \
  --vus 200
```

### Stress Test

Find the breaking point by continuously increasing load:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --scenario stress \
  --duration 15m \
  --vus 500
```

### Soak Test

Test for memory leaks and performance degradation over time:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --scenario soak \
  --duration 2h \
  --vus 50
```

## Authentication & Headers

### Bearer Token Authentication

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --auth "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

### Custom Headers

Add multiple custom headers:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --headers "X-API-Key:abc123,X-Client-ID:client456,X-Request-ID:req-789"
```

### Combined Authentication and Headers

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --auth "Bearer token123" \
  --headers "X-API-Version:2.0,X-Environment:staging"
```

## Operation Filtering

### Test Specific Endpoints

Test only GET and POST operations on the /users endpoint:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --operations "GET /users,POST /users"
```

### Test Multiple Endpoints

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --operations "GET /users,POST /users,GET /users/{id},PUT /users/{id}"
```

### Wildcard Filtering

Test all endpoints under /api/v1:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --operations "GET /api/v1/*,POST /api/v1/*"
```

## Script Generation

### Generate k6 Script Without Running

Create a k6 script for manual execution or customization:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --generate-only \
  --script-output tests/load/api-bench.js
```

Then run manually:

```bash
k6 run tests/load/api-bench.js
```

### Generate Script with Custom Scenario

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --scenario spike \
  --duration 10m \
  --vus 200 \
  --generate-only \
  --script-output spike-test.js
```

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: API Performance Tests

on:
  push:
    branches: [main]
  pull_request:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM

jobs:
  load-test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install k6
        run: |
          wget https://github.com/grafana/k6/releases/download/v0.48.0/k6-v0.48.0-linux-amd64.tar.gz
          tar -xzf k6-v0.48.0-linux-amd64.tar.gz
          sudo mv k6-v0.48.0-linux-amd64/k6 /usr/local/bin/

      - name: Build MockForge
        run: cargo build --release

      - name: Run load test
        run: |
          ./target/release/mockforge bench \
            --spec specs/api.yaml \
            --target ${{ secrets.API_URL }} \
            --duration 2m \
            --vus 50 \
            --threshold-ms 500 \
            --max-error-rate 0.05 \
            --auth "Bearer ${{ secrets.API_TOKEN }}" \
            --output bench-results/

      - name: Upload results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: load-test-results
          path: bench-results/

      - name: Check thresholds
        run: |
          # Parse results and fail if thresholds are breached
          # Implementation depends on your specific needs
```

### GitLab CI Pipeline

```yaml
load-test:
  stage: test
  image: rust:latest
  before_script:
    - apt-get update
    - apt-get install -y wget
    - wget https://github.com/grafana/k6/releases/download/v0.48.0/k6-v0.48.0-linux-amd64.tar.gz
    - tar -xzf k6-v0.48.0-linux-amd64.tar.gz
    - mv k6-v0.48.0-linux-amd64/k6 /usr/local/bin/
    - cargo build --release
  script:
    - ./target/release/mockforge bench
        --spec specs/api.yaml
        --target $API_URL
        --duration 2m
        --vus 50
        --auth "Bearer $API_TOKEN"
        --output bench-results/
  artifacts:
    paths:
      - bench-results/
    expire_in: 1 week
```

## Advanced Configurations

### Custom Thresholds

Set strict performance requirements:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --threshold-percentile p99 \
  --threshold-ms 1000 \
  --max-error-rate 0.01
```

### Verbose Output

Enable detailed logging:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --verbose
```

### Custom Output Directory

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --output results/$(date +%Y%m%d_%H%M%S)/
```

### Pre-production Testing

Test against staging with production-like load:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://staging.api.com \
  --scenario stress \
  --duration 30m \
  --vus 500 \
  --threshold-percentile p95 \
  --threshold-ms 500 \
  --max-error-rate 0.02 \
  --auth "Bearer $STAGING_TOKEN" \
  --headers "X-Environment:staging" \
  --output staging-load-test-$(date +%Y%m%d)/
```

### Microservices Testing

Test multiple services in parallel:

```bash
# Service 1
mockforge bench \
  --spec users-api.yaml \
  --target https://users.api.com \
  --output results/users/ &

# Service 2
mockforge bench \
  --spec orders-api.yaml \
  --target https://orders.api.com \
  --output results/orders/ &

# Service 3
mockforge bench \
  --spec payments-api.yaml \
  --target https://payments.api.com \
  --output results/payments/ &

wait
```

## Comparing Results

### Baseline vs Current

```bash
# Establish baseline
mockforge bench \
  --spec api.yaml \
  --target https://api.com \
  --output results/baseline/

# Test after changes
mockforge bench \
  --spec api.yaml \
  --target https://api.com \
  --output results/current/

# Compare (manual analysis or custom script)
diff results/baseline/summary.json results/current/summary.json
```

## Troubleshooting

### Debug Mode

If tests are failing, enable verbose mode:

```bash
mockforge bench \
  --spec api.yaml \
  --target https://api.example.com \
  --verbose \
  --generate-only \
  --script-output debug.js
```

Then inspect the generated script and run k6 manually:

```bash
k6 run --verbose debug.js
```

### Test Connectivity

Before running full load tests, verify the target is accessible:

```bash
curl -I https://api.example.com
```

### Check k6 Installation

```bash
k6 version
```

## Best Practices

1. **Start Small**: Begin with low VUs and short duration, then scale up
2. **Monitor Target**: Watch server metrics while running tests
3. **Use Staging**: Test against staging before production
4. **Version Control Scripts**: Save generated k6 scripts for reproducibility
5. **Document Baselines**: Keep historical results for comparison
6. **Automate**: Integrate load tests into CI/CD pipelines
7. **Set Realistic Thresholds**: Base thresholds on actual requirements

## Additional Resources

- [k6 Documentation](https://k6.io/docs/)
- [Load Testing Best Practices](https://k6.io/docs/testing-guides/)
- [OpenAPI Specification](https://spec.openapis.org/oas/latest.html)
