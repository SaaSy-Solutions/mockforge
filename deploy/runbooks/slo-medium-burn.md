# MockForge SLO Medium Burn Rate

## Alert
**Name**: `MockForgeSLOMediumBurn`
**Severity**: Warning
**Condition**: Error budget burning at 6x rate (entire monthly budget exhausted in ~5 days)

## Impact
Error budget is being consumed at an elevated rate. Action needed within hours, not minutes.

## Investigation

1. **Check Grafana SLO dashboard** for burn rate trends
2. **Identify the contributing errors or latency spikes**
3. **Check for slow degradation** (memory leak, connection pool exhaustion, disk filling up)

## Remediation

1. **Investigate and fix root cause** — this is not yet critical but will become one
2. **Freeze risky deployments** until the rate normalizes
3. See [SLO Breach runbook](slo-breach.md) for error budget policy

## Escalation
If burn rate doesn't decrease within 1 hour, escalate to the team lead.
