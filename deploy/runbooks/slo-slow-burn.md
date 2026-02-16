# MockForge SLO Slow Burn Rate

## Alert
**Name**: `MockForgeSLOSlowBurn`
**Severity**: Info
**Condition**: Error budget burning at >1x rate over 3 days

## Impact
Error budget is being consumed at a rate that will eventually exhaust it within the monthly window. This is a trend indicator, not an immediate problem.

## Investigation

1. **Review error trends** over the past week in Grafana
2. **Look for patterns** — specific times, endpoints, or operations with higher error rates
3. **Check for gradual degradation** (growing memory usage, increasing response times)

## Remediation

1. **Track as a reliability work item** — no immediate action required
2. **Schedule investigation** in the next sprint if the trend persists
3. **Review recent changes** that might have introduced subtle regressions

## Escalation
No immediate escalation. Create a ticket for reliability investigation.
