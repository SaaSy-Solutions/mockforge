# Pillar Query Guide

**Version:** 1.0.0
**Last Updated:** 2025-01-27

This guide explains how to query pillar-based metrics for test coverage and production usage analysis.

## Overview

With pillar tagging in place, you can now:

- **Query test coverage by pillar**: "Show me test coverage by pillar"
- **Query production usage by pillar**: "Which pillars are most used in production?"

## Test Coverage by Pillar

### Using the Coverage Script

The `pillar-coverage.sh` script generates test coverage reports grouped by pillar:

```bash
# Generate text report for all pillars
./scripts/pillar-coverage.sh

# Generate JSON report
./scripts/pillar-coverage.sh --format json

# Filter by specific pillar
./scripts/pillar-coverage.sh --pillar reality

# Custom output directory
./scripts/pillar-coverage.sh --output-dir ./my-coverage
```

### What It Does

1. **Parses pillar tags** from source files (looks for `Pillars: [Reality][AI]` in doc comments)
2. **Maps test files** to source modules
3. **Groups by pillar** and generates coverage reports

### Output Format

The script generates reports showing:
- Files tagged with each pillar
- File counts per pillar
- Test file mappings (when available)

### Example Output

```
MockForge Test Coverage by Pillar
==================================

Generated: 2025-01-27 10:00:00

Pillar: reality
----------------
  Files tagged: 5
  Tagged files:
    - crates/mockforge-core/src/reality.rs
    - crates/mockforge-core/src/chaos_utilities.rs
    - crates/mockforge-core/src/latency.rs
    - crates/mockforge-core/src/reality_continuum/mod.rs
    - crates/mockforge-core/src/generative_schema/mod.rs

Pillar: contracts
----------------
  Files tagged: 4
  Tagged files:
    - crates/mockforge-core/src/validation.rs
    - crates/mockforge-core/src/contract_validation.rs
    - crates/mockforge-core/src/contract_drift/mod.rs
    - crates/mockforge-core/src/schema_diff.rs
...
```

## Production Usage by Pillar

### Using the Usage Script

The `pillar-usage.sh` script queries Prometheus metrics to show pillar usage in production:

```bash
# Query default Prometheus instance (http://localhost:9090)
./scripts/pillar-usage.sh

# Query custom Prometheus URL
./scripts/pillar-usage.sh --prometheus-url http://prometheus.example.com:9090

# Query with custom time range
./scripts/pillar-usage.sh --time-range 24h

# Generate JSON output
./scripts/pillar-usage.sh --format json

# Filter by specific pillar
./scripts/pillar-usage.sh --pillar reality
```

### What It Queries

The script queries the following Prometheus metrics:

1. **Requests by pillar**: `sum(rate(mockforge_requests_total[1h])) by (pillar)`
2. **Error rate by pillar**: `sum(rate(mockforge_errors_total[1h])) by (pillar)`
3. **Average latency by pillar**: Average request duration by pillar

### Prerequisites

1. **Prometheus must be running** and accessible
2. **Metrics must have pillar labels** (use `record_*_with_pillar` methods)
3. **jq must be installed** for JSON parsing

### Example Output

```
MockForge Pillar Usage Report
=============================

Prometheus URL: http://localhost:9090
Time Range: 1h
Generated: 2025-01-27 10:00:00

Requests by Pillar (requests/second)
------------------------------------
  reality:        45.2 req/s
  contracts:      32.1 req/s
  devx:           12.5 req/s
  cloud:          8.3 req/s
  ai:             5.7 req/s

Error Rate by Pillar (errors/second)
------------------------------------
  reality:        0.1 err/s
  contracts:      0.2 err/s
  devx:           0.0 err/s
  cloud:          0.0 err/s
  ai:             0.1 err/s

Average Latency by Pillar (seconds)
-----------------------------------
  reality:        0.045 s
  contracts:      0.032 s
  devx:           0.012 s
  cloud:          0.008 s
  ai:             0.125 s
```

## Prometheus Queries

You can also query Prometheus directly using PromQL:

### Total Requests by Pillar

```promql
sum(rate(mockforge_requests_total[1h])) by (pillar)
```

### Error Rate by Pillar

```promql
sum(rate(mockforge_errors_total[1h])) by (pillar)
```

### Average Latency by Pillar

```promql
avg(rate(mockforge_request_duration_seconds_sum[1h])) by (pillar)
/
avg(rate(mockforge_request_duration_seconds_count[1h])) by (pillar)
```

### Request Distribution by Pillar

```promql
sum(mockforge_requests_total) by (pillar) / sum(mockforge_requests_total)
```

### Top Pillars by Request Volume

```promql
topk(5, sum(rate(mockforge_requests_total[1h])) by (pillar))
```

## Integration with CI/CD

### Coverage Reports in CI

Add to your CI pipeline:

```yaml
# .github/workflows/coverage.yml
- name: Generate pillar coverage report
  run: |
    ./scripts/pillar-coverage.sh --format json --output-dir ./coverage-reports
    # Upload to artifact storage or coverage service
```

### Usage Monitoring

Set up periodic usage reports:

```bash
# Cron job to generate daily usage reports
0 0 * * * /path/to/scripts/pillar-usage.sh --time-range 24h --format json > /var/log/pillar-usage-$(date +\%Y-\%m-\%d).json
```

## Troubleshooting

### No Pillar Data in Metrics

If pillar labels show as "unknown":

1. **Check metric recording**: Ensure you're using `record_*_with_pillar` methods
2. **Verify pillar tags**: Check that modules are tagged with `Pillars: [Pillar]`
3. **Check Prometheus**: Verify metrics are being scraped correctly

### Coverage Script Issues

If the coverage script fails:

1. **Install cargo-llvm-cov**: `cargo install cargo-llvm-cov`
2. **Check file paths**: Ensure source files are in `crates/` directory
3. **Verify pillar tags**: Check that files have `Pillars:` in doc comments

### Prometheus Connection Issues

If Prometheus queries fail:

1. **Check URL**: Verify Prometheus is accessible at the specified URL
2. **Check metrics**: Ensure `mockforge_requests_total` metric exists
3. **Check time range**: Verify the time range is valid (e.g., `1h`, `24h`, `7d`)

## Best Practices

1. **Regular Monitoring**: Set up periodic usage reports to track pillar adoption
2. **Coverage Goals**: Set coverage targets per pillar (e.g., 80% for Reality, 90% for Contracts)
3. **Alert on Imbalances**: Alert if one pillar has significantly higher error rates
4. **Document Findings**: Share pillar usage insights with the team

## References

- [Pillar Tagging Guide](PILLAR_TAGGING.md) - How to tag modules with pillars
- [PILLARS.md](../PILLARS.md) - Complete pillar documentation
- [Prometheus Metrics Guide](../PROMETHEUS_METRICS.md) - Prometheus metrics documentation
