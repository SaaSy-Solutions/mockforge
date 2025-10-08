# MockForge Service Level Objectives (SLOs)

## Overview

This document defines the Service Level Objectives (SLOs), Service Level Indicators (SLIs), and error budgets for MockForge.

## Service Level Indicators (SLIs)

### 1. Availability
- **Definition**: Percentage of successful HTTP requests (2xx status codes)
- **Measurement**: `(count of 2xx responses) / (total requests)`
- **Time Windows**: 5m, 30m, 1h, 30d

### 2. Latency
- **Definition**: Request duration from receipt to response
- **Measurement**: Histogram percentiles (p50, p95, p99)
- **Time Windows**: 5m rolling window

### 3. Error Rate
- **Definition**: Percentage of failed HTTP requests (5xx status codes)
- **Measurement**: `(count of 5xx responses) / (total requests)`
- **Time Windows**: 5m rolling window

## Service Level Objectives (SLOs)

### Availability SLO
- **Target**: 99.9% (three nines)
- **Window**: 30 days rolling
- **Error Budget**: 0.1% of requests
  - For 1M requests/month: 1,000 failed requests allowed
  - For 10M requests/month: 10,000 failed requests allowed
- **Impact of Breach**: User-facing errors, potential customer churn

### Latency SLO
- **Target**: 95% of requests complete in < 200ms
- **Window**: 5 minutes rolling
- **Error Budget**: 5% of requests may exceed 200ms
- **Impact of Breach**: Degraded user experience, slower integrations

## Error Budget Policy

### Error Budget Calculation
Error budget represents the allowed failure rate over the measurement window:
- **Availability Error Budget**: (1 - 0.999) = 0.1% of requests
- **30-day window**: ~43 minutes of downtime allowed per month

### Multi-Burn-Rate Alerts

We use multi-window, multi-burn-rate alerting to detect SLO violations early:

| Alert Type | Burn Rate | Detection Window | Severity | Response Time |
|------------|-----------|------------------|----------|---------------|
| Fast Burn | 14.4x | 1 hour | Critical | Page on-call immediately |
| Medium Burn | 6x | 6 hours | Warning | Investigate within 1 hour |
| Slow Burn | 1x | 3 days | Info | Review during business hours |

### Fast Burn (Critical)
- **Condition**: Error budget consumed at 14.4x normal rate
- **Impact**: Entire monthly budget exhausted in ~2 hours
- **Response**: Page on-call engineer immediately
- **Action**: Rollback recent changes, investigate root cause

### Medium Burn (Warning)
- **Condition**: Error budget consumed at 6x normal rate
- **Impact**: Entire monthly budget exhausted in ~5 days
- **Response**: Investigate within 1 hour
- **Action**: Identify trending issues, plan mitigation

### Slow Burn (Info)
- **Condition**: Error budget consumed at 1x rate
- **Impact**: Normal consumption rate
- **Response**: Review during business hours
- **Action**: Monitor trends, no immediate action required

## Error Budget Depletion Response

When error budget is depleted or at risk:

### 1. Error Budget < 25% Remaining
- **Action**: Feature freeze - no new features deployed
- **Focus**: Bug fixes and reliability improvements only
- **Review**: Daily standup on reliability improvements

### 2. Error Budget < 10% Remaining
- **Action**: Code freeze - only critical bug fixes
- **Focus**: Root cause analysis and fixes
- **Review**: Twice-daily incident response meetings

### 3. Error Budget Exhausted (0% Remaining)
- **Action**: Complete deployment freeze
- **Focus**: Incident resolution and postmortem
- **Review**: Continuous incident response until SLO restored

## SLO Monitoring

### Dashboards
- **MockForge Overview Dashboard**: Real-time SLI metrics
- **SLO Dashboard**: Error budget tracking and burn rate visualization

### Prometheus Queries

#### Current Availability SLI
```promql
sum(rate(mockforge_http_requests_total{status=~"2.."}[5m]))
/
sum(rate(mockforge_http_requests_total[5m]))
```

#### Error Budget Remaining (30d)
```promql
slo:mockforge:availability:error_budget_remaining
```

#### Current p95 Latency
```promql
histogram_quantile(0.95,
  sum(rate(mockforge_http_request_duration_seconds_bucket[5m])) by (le)
)
```

## Runbook Links

- [SLO Fast Burn Response](./runbooks/slo-fast-burn.md)
- [SLO Medium Burn Response](./runbooks/slo-medium-burn.md)
- [Error Budget Exhausted Response](./runbooks/slo-budget-exhausted.md)

## Review and Updates

SLOs should be reviewed quarterly and updated based on:
- User requirements and expectations
- Business objectives
- Historical performance data
- Cost of maintaining higher reliability

**Last Updated**: 2025-10-07
**Next Review**: 2026-01-07
