# MockForge SLO Fast Burn Rate

## Alert
**Name**: `MockForgeSLOFastBurn`
**Severity**: Critical
**Condition**: Error budget burning at 14.4x rate (entire monthly budget exhausted in ~2 hours)

## Impact
Critical reliability degradation. At this rate, the entire monthly error budget will be consumed within 2 hours.

## Immediate Actions

1. **Page on-call immediately** — this is a critical incident
2. **Check for active incidents** in dependent services
3. **Check recent deployments** and rollback if suspect
   ```bash
   kubectl rollout undo deployment/mockforge
   ```

## Investigation

Follow the standard investigation from:
- [High Error Rate runbook](high-error-rate.md)
- [High Latency runbook](high-latency.md)
- [SLO Breach runbook](slo-breach.md)

## Escalation
This alert should page immediately. Treat as a P1 incident.
