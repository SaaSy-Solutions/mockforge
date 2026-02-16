# MockForge High CPU Usage

## Alert
**Name**: `MockForgeHighCPUUsage`
**Severity**: Warning
**Condition**: CPU usage exceeds 80% for 10 minutes

## Impact
High CPU may cause increased latency and potential request timeouts.

## Investigation Steps

1. **Check CPU metrics**
   ```bash
   kubectl top pods -l app=mockforge
   ```

2. **Check request rate**
   - Has traffic increased significantly?
   - Check `mockforge_http_requests_total` rate

3. **Check for expensive operations**
   - Template expansion with complex Handlebars templates
   - Schema validation on large payloads
   - Regex matching in route resolution

## Remediation

1. **Traffic spike**: Scale horizontally (increase replicas)
2. **Expensive operations**: Profile and optimize hot paths
3. **Increase limits**: Update CPU limits in deployment config
4. **Rate limiting**: Enable or tighten rate limits to protect the service
