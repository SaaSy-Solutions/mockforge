# MockForge PVC Almost Full

## Alert
**Name**: `MockForgePVCAlmostFull`
**Severity**: Warning
**Condition**: A replica's recorder PVC usage exceeds 85% for 10 minutes (each replica has its own claim: `recorder-db-mockforge-N` from `k8s/`, `data-<release>-N` from Helm; the alert's `persistentvolumeclaim` label names it)

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
2. **Expand PVC**: Resize the replica's claim (if the storage class supports it). Also raise the size in the StatefulSet's `volumeClaimTemplates` (or Helm `persistence.size`) for future replicas; that field cannot be patched in place, so recreate the StatefulSet with `kubectl delete statefulset mockforge --cascade=orphan` and re-apply.
   ```bash
   kubectl patch pvc recorder-db-mockforge-0 -p '{"spec":{"resources":{"requests":{"storage":"20Gi"}}}}'
   ```
3. **Reduce recording volume**: Disable recording for high-traffic endpoints
4. **Enable compression**: Compress recording files
