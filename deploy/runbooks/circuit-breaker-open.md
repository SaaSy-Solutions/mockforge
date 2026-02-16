# MockForge Circuit Breaker Open

## Alert
**Name**: `MockForgeCircuitBreakerOpen`
**Severity**: Warning
**Condition**: A circuit breaker has been open for 5 minutes

## Impact
The circuit breaker has tripped for an external service, meaning requests to that service are being rejected without attempting the call. This protects the system but degrades functionality.

## Investigation Steps

1. **Identify which service**
   - Check `mockforge_circuit_breaker_state` label for service name
   - Common services: `redis`, `s3`, `email`, `database`

2. **Check the downstream service**
   - Is the service healthy?
   - Are other consumers also having issues?

3. **Check error logs**
   ```bash
   kubectl logs -l app=mockforge --since=10m | grep "circuit breaker"
   ```

## Remediation

1. **Downstream service down**: Fix the downstream service, circuit breaker will auto-recover
2. **Network issues**: Check network policies, DNS resolution, service endpoints
3. **Configuration error**: Verify connection strings and credentials
4. **Manual reset**: If the downstream service is recovered but the circuit breaker hasn't closed, restart the pod
