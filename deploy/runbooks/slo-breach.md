# MockForge SLO Breach

## Alert
**Name**: `MockForgeSLOAvailabilityBreach` / `MockForgeSLOLatencyBreach`
**Severity**: Critical (availability) / Warning (latency)
**Condition**: Availability below 99.9% or p95 latency above 200ms for 5 minutes

## Impact
Service Level Objectives are being violated. If sustained, this will exhaust the monthly error budget.

## Investigation Steps

1. **Check error budget remaining**
   - Grafana SLO dashboard: `slo:mockforge:availability:error_budget_remaining`
   - If budget is >50% remaining, this may be a transient spike

2. **Identify root cause**
   - Follow the [High Error Rate runbook](high-error-rate.md) if availability is the issue
   - Follow the [High Latency runbook](high-latency.md) if latency is the issue

3. **Check recent deployments**
   ```bash
   kubectl rollout history deployment/mockforge
   ```

4. **Check for external factors**
   - Upstream dependency outages
   - Infrastructure incidents
   - Traffic pattern changes

## Remediation

1. **Rollback** if a recent deployment caused the breach
2. **Scale up** if traffic has increased beyond capacity
3. **Fix root cause** using the specific error/latency runbook
4. **Freeze deployments** if error budget is nearly exhausted

## Error Budget Policy

- **>50% remaining**: Monitor, no action required
- **25-50% remaining**: Freeze risky deployments, focus on reliability work
- **<25% remaining**: All hands on reliability, no feature deployments
- **Exhausted**: Incident response, post-mortem required
