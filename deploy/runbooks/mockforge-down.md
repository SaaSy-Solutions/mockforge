# MockForge Down

## Alert
**Name**: `MockForgeDown`
**Severity**: Critical
**Condition**: `up{job="mockforge"} == 0` for 1 minute

## Impact
MockForge instance is completely unreachable. Mock API responses are unavailable for any services depending on this instance.

## Investigation Steps

1. **Check pod/container status**
   ```bash
   kubectl get pods -l app=mockforge -o wide
   kubectl describe pod <pod-name>
   docker ps --filter name=mockforge
   ```

2. **Check logs for crash/OOM**
   ```bash
   kubectl logs <pod-name> --previous
   kubectl top pod -l app=mockforge
   ```

3. **Check node health**
   ```bash
   kubectl get nodes
   kubectl describe node <node-name>
   ```

4. **Check resource limits**
   ```bash
   kubectl get pod <pod-name> -o jsonpath='{.spec.containers[*].resources}'
   ```

## Remediation

1. **Pod in CrashLoopBackOff**: Check logs for startup errors (missing env vars, DB connection failures)
2. **OOMKilled**: Increase memory limits in deployment manifest or Helm values
3. **Node issues**: Cordon unhealthy node, pods should reschedule automatically
4. **Network issues**: Check service endpoints and network policies
5. **Manual restart**: `kubectl rollout restart deployment/mockforge`

## Escalation
If not resolved within 5 minutes, page the on-call engineer.
