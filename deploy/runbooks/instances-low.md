# MockForge Instances Low

## Alert
**Name**: `MockForgeInstancesLow`
**Severity**: Warning
**Condition**: Fewer than 2 MockForge instances running for 5 minutes

## Impact
Reduced availability and no redundancy. If the remaining instance goes down, the service will be completely unavailable.

## Investigation Steps

1. **Check pod status**
   ```bash
   kubectl get pods -l app=mockforge
   kubectl get events --sort-by='.lastTimestamp' | grep mockforge
   ```

2. **Check HPA status** (if using autoscaling)
   ```bash
   kubectl get hpa
   ```

3. **Check node capacity**
   ```bash
   kubectl describe nodes | grep -A 5 "Allocated resources"
   ```

## Remediation

1. **Scale up manually** if needed: `kubectl scale deployment/mockforge --replicas=2`
2. **Fix scheduling issues**: Check for node affinity/anti-affinity rules, resource constraints
3. **Check for failed rollout**: `kubectl rollout status deployment/mockforge`
