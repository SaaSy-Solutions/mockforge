# MockForge Chaos Scenario Failed

## Alert
**Name**: `MockForgeChaosScenarioFailed`
**Severity**: Warning
**Condition**: A chaos engineering scenario has failed

## Impact
A resilience test has identified a failure mode. This may indicate a real vulnerability in the system.

## Investigation Steps

1. **Identify the failed scenario**
   - Check `mockforge_chaos_scenario_status` label for scenario name
   - Review scenario configuration and expected behavior

2. **Check if this is expected**
   - Was this a new scenario being tested?
   - Has this scenario passed before?

3. **Review failure details**
   ```bash
   kubectl logs -l app=mockforge --since=10m | grep -i chaos
   ```

## Remediation

1. **Expected failure**: Update scenario expectations or fix the identified resilience gap
2. **Unexpected failure**: Investigate root cause — the system may have regressed
3. **Disable scenario**: If causing issues, disable the failing scenario temporarily
