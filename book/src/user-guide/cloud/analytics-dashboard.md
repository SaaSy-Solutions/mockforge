# Analytics Dashboard

**Pillars:** [Cloud]

The Analytics Dashboard provides leadership insight into coverage, risk, and usage. It shows which scenarios are used most, what personas are hit by CI, which endpoints are under-tested, which mocks have stale reality levels, and what percentage of mocks are drifting from real data.

## Overview

The Analytics Dashboard gives you:

- **Scenario Usage Heatmaps**: Which scenarios are used most, usage patterns over time
- **Persona CI Hit Tracking**: Which personas are hit by CI, persona usage frequency
- **Endpoint Coverage Analysis**: Which endpoints are under-tested, test coverage per endpoint
- **Reality Level Staleness**: Which mocks have stale reality levels, recommendations for updates
- **Drift Percentage Tracking**: What percentage of mocks are drifting from real data, drift trends

## Scenario Usage Heatmaps

### Overview

Visualize which scenarios are used most frequently:

```
Scenario                    Usage Count    Last Used
─────────────────────────────────────────────────────
checkout-success           1,234          2025-01-27
payment-failure            892           2025-01-27
cart-abandonment           567           2025-01-26
user-signup                445           2025-01-25
```

### Usage Patterns

View usage patterns over time:
- Peak usage times
- Usage trends (increasing/decreasing)
- Seasonal patterns

### Access

```bash
# Get scenario usage metrics
GET /api/v2/analytics/scenarios/usage?workspace_id=workspace-123&time_range=30d
```

## Persona CI Hit Tracking

### Overview

Track which personas are used in CI/CD pipelines:

```
Persona                    CI Hits    Last Hit
───────────────────────────────────────────────
premium-customer           234        2025-01-27
fraud-suspect              156        2025-01-27
new-user                   89        2025-01-26
churned-user               45        2025-01-25
```

### Insights

- **Coverage Gaps**: Personas not hit by CI
- **Usage Frequency**: How often personas are used
- **CI Integration**: Which CI systems use which personas

### Access

```bash
# Get persona CI hits
GET /api/v2/analytics/personas/ci-hits?workspace_id=workspace-123
```

## Endpoint Coverage Analysis

### Overview

Identify which endpoints are under-tested:

```
Endpoint                    Test Count    Last Tested    Coverage
─────────────────────────────────────────────────────────────────
GET /api/users/{id}         45           2025-01-27     100%
POST /api/orders            23           2025-01-26     85%
GET /api/products           12           2025-01-25     60%  ⚠️
DELETE /api/users/{id}      5            2025-01-20     30%  ⚠️
```

### Coverage Metrics

- **Test Count**: Number of tests covering endpoint
- **Last Tested**: When endpoint was last tested
- **Coverage Percentage**: Test coverage score
- **Missing Scenarios**: Scenarios that should exist but don't

### Access

```bash
# Get endpoint coverage
GET /api/v2/analytics/endpoints/coverage?workspace_id=workspace-123&min_coverage=80
```

## Reality Level Staleness

### Overview

Track which mocks have stale reality levels:

```
Endpoint                    Current Level    Last Updated    Staleness
─────────────────────────────────────────────────────────────────────
GET /api/users/{id}        3               2025-01-15      12 days  ⚠️
POST /api/orders           2               2025-01-20      7 days
GET /api/products          4               2025-01-27      0 days
```

### Recommendations

- **Update Needed**: Mocks with stale reality levels
- **Update Priority**: Based on usage and importance
- **Update Suggestions**: Recommended reality level updates

### Access

```bash
# Get reality level staleness
GET /api/v2/analytics/reality-levels/staleness?workspace_id=workspace-123&max_staleness_days=30
```

## Drift Percentage Tracking

### Overview

Track what percentage of mocks are drifting from real data:

```
Metric                      Value    Trend
───────────────────────────────────────────
Total Mocks                150      ─
Drifting Mocks             45       ↑
Drift Percentage           30%      ↑
High-Drift Mocks           12       ↑
```

### Drift Trends

View drift trends over time:
- Increasing drift (mocks diverging from real)
- Decreasing drift (mocks converging with real)
- Stable drift (consistent divergence)

### Access

```bash
# Get drift percentage
GET /api/v2/analytics/drift/percentage?workspace_id=workspace-123
```

## Dashboard Views

### Overview Dashboard

High-level metrics:
- Total scenarios, personas, endpoints
- Overall coverage percentage
- Average reality level staleness
- Overall drift percentage

### Detailed Views

Drill down into specific areas:
- Scenario usage details
- Persona usage breakdown
- Endpoint coverage details
- Reality level analysis
- Drift analysis

## Configuration

### Enable Analytics

```yaml
# mockforge.yaml
analytics:
  enabled: true
  collection_interval: 60  # seconds
  retention_days: 90
```

### Coverage Thresholds

```yaml
analytics:
  coverage:
    min_coverage_percentage: 80
    alert_on_low_coverage: true
```

### Staleness Thresholds

```yaml
analytics:
  staleness:
    max_staleness_days: 30
    alert_on_stale: true
```

## Best Practices

1. **Review Regularly**: Check dashboard weekly/monthly
2. **Set Thresholds**: Configure coverage and staleness thresholds
3. **Act on Insights**: Use insights to improve testing and mocks
4. **Track Trends**: Monitor trends over time
5. **Share with Team**: Share insights with development teams

## Related Documentation

- [MockOps Pipelines](mockops-pipelines.md) - Pipeline automation
- [Federation](federation.md) - Multi-workspace federation
- [Cloud Workspaces](cloud-workspaces.md) - Workspace management

