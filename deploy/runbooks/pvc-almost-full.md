# MockForge PVC Almost Full

## Alert
**Name**: `MockForgePVCAlmostFull`
**Severity**: Warning
**Condition**: Recorder PVC usage exceeds 85% for 10 minutes

## Impact
The traffic recording persistent volume is running out of space. New recordings may fail.

## Investigation Steps

1. **Check current usage**
   ```bash
   kubectl exec -it <pod-name> -- df -h /data
   ```

2. **Check recording files**
   ```bash
   kubectl exec -it <pod-name> -- ls -lhS /data/recordings/ | head -20
   ```

3. **Check retention policy**
   - Are old recordings being cleaned up automatically?

## Remediation

1. **Clean old recordings**: Delete recordings older than retention period
2. **Expand PVC**: Resize the persistent volume claim (if storage class supports it)
   ```bash
   kubectl patch pvc mockforge-recorder-pvc -p '{"spec":{"resources":{"requests":{"storage":"20Gi"}}}}'
   ```
3. **Reduce recording volume**: Disable recording for high-traffic endpoints
4. **Enable compression**: Compress recording files
