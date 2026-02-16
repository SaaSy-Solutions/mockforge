# MockForge High Latency

## Alert
**Name**: `MockForgeHighLatency` / `MockForgeCriticalLatency`
**Severity**: Warning (>500ms) / Critical (>1s)
**Condition**: p95 request latency exceeds threshold for 5m/2m

## Impact
Slow mock API responses may cause timeouts in dependent services running integration tests or development workflows.

## Investigation Steps

1. **Check current latency metrics**
   ```bash
   curl -s localhost:9090/metrics | grep mockforge_http_request_duration
   ```

2. **Identify slow endpoints**
   - Check Grafana dashboard for per-endpoint latency breakdown
   - Look for specific paths with elevated p95/p99

3. **Check resource utilization**
   ```bash
   kubectl top pods -l app=mockforge
   ```

4. **Check database connection pool**
   - Look for `mockforge_connection_pool_active` approaching max
   - Check for slow queries in database logs

5. **Check for traffic spike**
   ```bash
   curl -s localhost:9090/metrics | grep mockforge_http_requests_total
   ```

## Remediation

1. **High CPU**: Scale horizontally (increase replica count) or vertically (increase CPU limits)
2. **Memory pressure**: Check for memory leaks, increase limits
3. **DB connection exhaustion**: Increase `DATABASE_MAX_CONNECTIONS` or optimize queries
4. **Traffic spike**: Enable rate limiting or scale out
5. **Template expansion**: Complex Handlebars templates may cause slowdowns — check response generation time

## Escalation
Critical latency (>1s) for more than 5 minutes should be treated as an incident.
