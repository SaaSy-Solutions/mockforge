# MockForge Connection Pool Near Exhaustion

## Alert
**Name**: `MockForgeConnectionPoolExhaustion`
**Severity**: Warning
**Condition**: Connection pool utilization exceeds 90% for 5 minutes

## Impact
The connection pool is nearly exhausted. New requests may queue or fail if all connections are in use.

## Investigation Steps

1. **Check active connections**
   ```bash
   curl -s localhost:9090/metrics | grep mockforge_connection_pool
   ```

2. **Check for slow queries or hung connections**
   ```sql
   SELECT pid, query, state, query_start FROM pg_stat_activity WHERE datname = 'mockforge' ORDER BY query_start;
   ```

3. **Check request rate**
   - Has traffic increased?
   - Are there long-running operations holding connections?

## Remediation

1. **Increase pool size**: Set `DATABASE_MAX_CONNECTIONS` to a higher value (default: 20)
2. **Kill slow queries**: Terminate queries that have been running too long
   ```sql
   SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state = 'active' AND query_start < NOW() - INTERVAL '5 minutes';
   ```
3. **Scale horizontally**: More replicas distribute connection load
4. **Optimize queries**: Ensure queries use indexes and don't hold connections unnecessarily
