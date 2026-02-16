# MockForge SLO Error Budget Exhausted

## Alert
**Name**: `MockForgeSLOErrorBudgetExhausted`
**Severity**: Critical
**Condition**: Monthly error budget is fully consumed (0% remaining)

## Impact
The service has exhausted its reliability allowance for the current 30-day window. Any further errors increase the SLO violation.

## Immediate Actions

1. **Declare an incident** — this requires incident response
2. **Freeze all non-critical deployments**
3. **Redirect engineering effort** to reliability improvements

## Investigation

1. **Review the timeline** of error budget consumption
2. **Identify the primary contributors** (specific incidents, gradual degradation)
3. **Conduct a post-mortem** for the SLO violation

## Remediation

1. **Fix all known reliability issues** before resuming feature work
2. **Add safeguards** to prevent recurrence (canary deployments, better alerting thresholds)
3. **Review SLO targets** — if the target is unrealistic, adjust it with stakeholder agreement

## Post-Incident

- Write post-mortem within 48 hours
- Track action items to completion
- Review error budget policy with the team
