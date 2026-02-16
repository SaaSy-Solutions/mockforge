# MockForge High Memory Usage

## Alert
**Name**: `MockForgeHighMemoryUsage`
**Severity**: Warning
**Condition**: Process resident memory exceeds 400MB for 10 minutes

## Impact
High memory usage may lead to OOM kills, causing service disruption.

## Investigation Steps

1. **Check memory trend**
   - Is it a gradual increase (leak) or sudden spike?
   - Check Grafana for `process_resident_memory_bytes` over time

2. **Check active connections and requests**
   ```bash
   curl -s localhost:9090/metrics | grep mockforge_connection
   ```

3. **Check for large specs**
   - Large OpenAPI specs or many routes consume more memory
   - Check if a new spec was loaded recently

## Remediation

1. **Memory leak**: Restart the pod as a short-term fix, investigate root cause
2. **Large spec**: Consider splitting large OpenAPI specs
3. **Increase limits**: Update resource limits in Helm values or K8s manifests
4. **Scale horizontally**: Distribute load across more instances
