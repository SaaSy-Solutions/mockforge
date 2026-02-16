# MockForge High Error Rate

## Alert
**Name**: `MockForgeHighErrorRate`
**Severity**: Critical
**Condition**: 5xx error rate exceeds 5% for 5 minutes

## Impact
A significant portion of mock API requests are failing, which may block CI/CD pipelines, integration tests, or development workflows.

## Investigation Steps

1. **Check error distribution**
   ```bash
   curl -s localhost:9090/metrics | grep 'mockforge_http_requests_total{status="5'
   ```

2. **Check application logs for errors**
   ```bash
   kubectl logs -l app=mockforge --since=10m | grep -i error
   ```

3. **Identify error patterns**
   - Are errors concentrated on specific endpoints?
   - Are they related to specific OpenAPI specs?
   - Did a recent deployment introduce the issue?

4. **Check dependencies**
   - Database connectivity
   - Storage (S3) availability
   - Redis connectivity (if enabled)

## Remediation

1. **Bad deployment**: Roll back to previous version
   ```bash
   kubectl rollout undo deployment/mockforge
   ```
2. **Database issues**: Check connection, run `SELECT 1` health check
3. **Invalid spec**: Check if a recently loaded OpenAPI spec has issues
4. **Resource exhaustion**: Scale up or restart pods
5. **Circuit breaker open**: Check circuit breaker metrics, underlying service may be down

## Escalation
Sustained >5% error rate for 10+ minutes requires incident response.
