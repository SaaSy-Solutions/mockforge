# Automated Load Testing & Performance Regression - CI Guide

Complete guide for implementing automated load testing and performance regression tests in CI/CD.

## Table of Contents

- [Overview](#overview)
- [Load Testing Strategy](#load-testing-strategy)
- [Performance Benchmarks](#performance-benchmarks)
- [CI/CD Integration](#cicd-integration)
- [Regression Detection](#regression-detection)
- [Reporting](#reporting)

---

## Overview

Automated load testing and performance regression detection ensures MockForge maintains performance standards and catches regressions before release.

### Goals

- ✅ **Automated Load Tests**: Run on every PR and release
- ✅ **Performance Benchmarks**: Track performance over time
- ✅ **Regression Detection**: Alert on performance degradation
- ✅ **Performance Budgets**: Enforce performance limits
- ✅ **Historical Tracking**: Compare against baseline

---

## Load Testing Strategy

### Test Scenarios

**1. Standard Load Tests (CI)**
- Quick validation (< 5 minutes)
- Moderate load (100-1000 concurrent users)
- Run on every PR

**2. Extended Load Tests (Nightly)**
- Comprehensive validation (15-30 minutes)
- High load (10,000+ concurrent users)
- Run nightly on main branch

**3. Stress Tests (Pre-Release)**
- Maximum capacity testing
- Failure point identification
- Run before major releases

### Load Profiles

**Standard Profile:**
```javascript
// tests/load/standard_load.js
export const options = {
  stages: [
    { duration: '1m', target: 100 },   // Ramp up
    { duration: '2m', target: 100 },   // Sustain
    { duration: '1m', target: 0 },     // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.01'],
  },
};
```

**Extended Profile:**
```javascript
// tests/load/extended_load.js
export const options = {
  stages: [
    { duration: '5m', target: 5000 },
    { duration: '3m', target: 5000 },
    { duration: '3m', target: 10000 },
    { duration: '5m', target: 10000 },
    { duration: '6m', target: 0 },
  ],
  thresholds: {
    http_req_duration: ['p(95)<1000', 'p(99)<2000'],
    http_req_failed: ['rate<0.01'],
  },
};
```

---

## Performance Benchmarks

### Benchmark Categories

**1. Startup Performance**
- Server startup time
- First request latency
- Memory usage at startup

**2. Request Performance**
- Latency percentiles (p50, p95, p99, p99.9)
- Throughput (requests/second)
- Error rates

**3. Resource Usage**
- CPU usage
- Memory usage
- Network I/O

**4. Protocol-Specific**
- HTTP request/response
- gRPC call latency
- WebSocket message latency
- GraphQL query performance

### Benchmark Implementation

```rust
// benches/performance/request_latency.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use mockforge_core::ServerConfig;

fn benchmark_request_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_latency");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                let server = start_test_server(config_with_size(size));
                b.iter(|| {
                    black_box(make_request(&server));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_request_latency);
criterion_main!(benches);
```

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/load-tests.yml

name: Load Tests & Performance

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  schedule:
    # Run extended tests nightly
    - cron: '0 2 * * *'

jobs:
  standard-load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build MockForge
        run: cargo build --release

      - name: Start MockForge Server
        run: |
          ./target/release/mockforge serve --http-port 3000 &
          sleep 5

      - name: Run Standard Load Tests
        uses: grafana/k6-action@v0.3.0
        with:
          filename: tests/load/standard_load.js
          cloud: false

      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: load-test-results
          path: load-test-results.json

  extended-load-test:
    if: github.event_name == 'schedule' || github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build MockForge
        run: cargo build --release

      - name: Start MockForge Server
        run: |
          ./target/release/mockforge serve --http-port 3000 &
          sleep 5

      - name: Run Extended Load Tests
        uses: grafana/k6-action@v0.3.0
        with:
          filename: tests/load/extended_load.js
          cloud: false

      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: extended-load-test-results
          path: load-test-results.json

  performance-benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run Benchmarks
        run: cargo bench --bench performance

      - name: Compare with Baseline
        run: |
          # Compare current benchmarks with baseline
          cargo bench --bench performance -- --baseline main

      - name: Upload Benchmark Results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: target/criterion/

  performance-regression:
    needs: [performance-benchmarks]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Download Benchmark Results
        uses: actions/download-artifact@v3
        with:
          name: benchmark-results

      - name: Check for Regressions
        run: |
          # Compare against baseline
          # Fail if performance degraded > 10%
          python scripts/check_performance_regression.py

      - name: Comment PR
        if: github.event_name == 'pull_request'
        uses: actions/github-script@v6
        with:
          script: |
            // Post performance comparison to PR
```

### Performance Regression Detection

```python
# scripts/check_performance_regression.py

import json
import sys
from pathlib import Path

PERFORMANCE_BUDGET = {
    'startup_time_ms': 100,
    'p95_latency_ms': 500,
    'p99_latency_ms': 1000,
    'throughput_rps': 1000,
}

REGRESSION_THRESHOLD = 0.10  # 10% degradation

def load_baseline():
    baseline_path = Path('benchmarks/baseline.json')
    if baseline_path.exists():
        return json.loads(baseline_path.read_text())
    return None

def load_current():
    results_path = Path('target/criterion/results.json')
    if results_path.exists():
        return json.loads(results_path.read_text())
    return None

def check_regression(baseline, current):
    regressions = []

    for metric, budget in PERFORMANCE_BUDGET.items():
        if metric in baseline and metric in current:
            baseline_value = baseline[metric]
            current_value = current[metric]

            # Check if exceeded budget
            if current_value > budget:
                regressions.append({
                    'metric': metric,
                    'current': current_value,
                    'budget': budget,
                    'type': 'budget_exceeded'
                })

            # Check for regression
            if current_value > baseline_value * (1 + REGRESSION_THRESHOLD):
                degradation = ((current_value - baseline_value) / baseline_value) * 100
                regressions.append({
                    'metric': metric,
                    'baseline': baseline_value,
                    'current': current_value,
                    'degradation_percent': degradation,
                    'type': 'regression'
                })

    return regressions

def main():
    baseline = load_baseline()
    current = load_current()

    if not current:
        print("No current benchmark results found")
        sys.exit(1)

    if baseline:
        regressions = check_regression(baseline, current)

        if regressions:
            print("Performance regressions detected:")
            for reg in regressions:
                print(f"  - {reg['metric']}: {reg}")
            sys.exit(1)
        else:
            print("No performance regressions detected")
    else:
        print("No baseline found, skipping regression check")
        # Save current as baseline
        Path('benchmarks/baseline.json').write_text(json.dumps(current, indent=2))

if __name__ == '__main__':
    main()
```

---

## Regression Detection

### Performance Budgets

```yaml
# .github/performance-budgets.yml

budgets:
  startup:
    max_time_ms: 100
    max_memory_mb: 50

  http:
    p50_latency_ms: 50
    p95_latency_ms: 500
    p99_latency_ms: 1000
    max_error_rate: 0.01

  grpc:
    p50_latency_ms: 20
    p95_latency_ms: 200
    p99_latency_ms: 500
    max_error_rate: 0.01

  websocket:
    connection_time_ms: 100
    message_latency_ms: 50
    max_error_rate: 0.01
```

### Baseline Management

```bash
# Save baseline after successful release
cargo bench --bench performance -- --save-baseline release-0.2.8

# Compare against baseline
cargo bench --bench performance -- --baseline release-0.2.8

# Update baseline
cargo bench --bench performance -- --baseline release-0.2.8 --save-baseline release-0.2.9
```

---

## Reporting

### Performance Dashboard

**Metrics to Track:**

- Request latency (p50, p95, p99, p99.9)
- Throughput (requests/second)
- Error rates
- Resource usage (CPU, memory)
- Startup time

**Visualization:**

- Time-series graphs
- Comparison charts (current vs baseline)
- Regression alerts
- Performance trends

### Test Reports

**Load Test Reports:**

```json
{
  "timestamp": "2024-01-01T00:00:00Z",
  "test_type": "standard_load",
  "duration_seconds": 240,
  "metrics": {
    "http_req_duration": {
      "p50": 45,
      "p95": 420,
      "p99": 850,
      "p99.9": 1200
    },
    "http_req_failed": 0.001,
    "throughput_rps": 1250
  },
  "thresholds": {
    "passed": true,
    "failed": []
  }
}
```

---

## Summary

Automated load testing and performance regression detection provides:

- ✅ **CI Integration**: Automated testing on every PR
- ✅ **Performance Benchmarks**: Track performance over time
- ✅ **Regression Detection**: Alert on degradation
- ✅ **Performance Budgets**: Enforce limits
- ✅ **Historical Tracking**: Compare against baseline

**Status**: Load testing infrastructure exists, CI integration needed

---

**Last Updated**: 2024-01-01
**Version**: 1.0
