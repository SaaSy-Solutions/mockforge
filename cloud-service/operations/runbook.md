# MockForge Cloud Service - Operations Runbook

**Purpose:** Day-to-day operations procedures for the managed SaaS service
**Last Updated:** 2025-01-27

---

## Table of Contents

1. [Daily Operations](#daily-operations)
2. [Deployment Procedures](#deployment-procedures)
3. [Monitoring & Alerts](#monitoring--alerts)
4. [Incident Response](#incident-response)
5. [Backup & Recovery](#backup--recovery)
6. [Scaling Procedures](#scaling-procedures)
7. [Maintenance Windows](#maintenance-windows)

---

## Daily Operations

### Morning Checklist

1. **Review Dashboards**
   - Check system health dashboard
   - Review error rates
   - Check resource utilization
   - Review overnight alerts

2. **Check Logs**
   - Review application logs for errors
   - Check security logs
   - Review access logs
   - Verify backup completion

3. **User Activity**
   - Review new signups
   - Check active user count
   - Review support tickets
   - Monitor usage trends

### Weekly Tasks

1. **Performance Review**
   - Analyze metrics trends
   - Identify optimization opportunities
   - Review capacity planning
   - Update forecasts

2. **Security Review**
   - Review access logs
   - Check for suspicious activity
   - Verify security patches
   - Review compliance status

3. **Backup Verification**
   - Test backup restoration
   - Verify backup integrity
   - Review retention policies
   - Document backup status

---

## Deployment Procedures

### Standard Deployment

1. **Pre-Deployment**
   ```bash
   # Run tests
   cargo test

   # Build artifacts
   cargo build --release

   # Create deployment package
   ./scripts/create-deployment-package.sh
   ```

2. **Staging Deployment**
   ```bash
   # Deploy to staging
   ./scripts/deploy-staging.sh

   # Run smoke tests
   ./scripts/smoke-tests.sh --environment staging

   # Verify functionality
   ```

3. **Production Deployment**
   ```bash
   # Create deployment ticket
   # Get approval

   # Deploy to production
   ./scripts/deploy-production.sh

   # Monitor deployment
   # Verify health checks
   # Run smoke tests
   ```

### Blue-Green Deployment

1. **Prepare Green Environment**
   - Deploy new version to green
   - Run health checks
   - Verify functionality

2. **Switch Traffic**
   - Update load balancer
   - Monitor metrics
   - Verify no errors

3. **Cleanup**
   - Keep blue for rollback
   - Monitor for 1 hour
   - Decommission blue if stable

### Rollback Procedure

1. **Identify Issue**
   - Check error rates
   - Review logs
   - Assess impact

2. **Execute Rollback**
   ```bash
   # Rollback to previous version
   ./scripts/rollback.sh

   # Verify rollback
   # Monitor metrics
   ```

3. **Post-Rollback**
   - Document incident
   - Root cause analysis
   - Fix and retest

---

## Monitoring & Alerts

### Key Metrics

**Availability**
- Target: 99.9% uptime
- Alert: < 99.5% for 5 minutes

**Latency**
- Target: P95 < 200ms
- Alert: P95 > 500ms for 5 minutes

**Error Rate**
- Target: < 0.1%
- Alert: > 1% for 5 minutes

**Resource Usage**
- CPU: Alert at 80%
- Memory: Alert at 85%
- Disk: Alert at 90%

### Alert Response

1. **Critical Alerts** (PagerDuty)
   - Immediate response required
   - On-call engineer notified
   - Escalate if no response in 15 min

2. **Warning Alerts** (Email)
   - Review within 1 hour
   - Investigate and resolve
   - Document if recurring

3. **Info Alerts** (Dashboard)
   - Review during daily check
   - Track trends
   - Address if needed

---

## Incident Response

### Severity Levels

**P0 - Critical**
- Service completely down
- Data loss or corruption
- Security breach
- Response: Immediate (< 15 min)

**P1 - High**
- Major feature broken
- Performance degradation
- Partial outage
- Response: < 1 hour

**P2 - Medium**
- Minor feature broken
- Non-critical errors
- Performance issues
- Response: < 4 hours

**P3 - Low**
- Cosmetic issues
- Enhancement requests
- Documentation updates
- Response: < 24 hours

### Incident Procedure

1. **Detection**
   - Alert received
   - User report
   - Monitoring detection

2. **Triage**
   - Assess severity
   - Assign owner
   - Create incident ticket
   - Notify team

3. **Response**
   - Investigate root cause
   - Implement fix
   - Verify resolution
   - Monitor stability

4. **Post-Incident**
   - Document incident
   - Root cause analysis
   - Action items
   - Process improvements

---

## Backup & Recovery

### Backup Schedule

- **Database**: Hourly incremental, daily full
- **Object Storage**: Daily snapshots
- **Configuration**: Daily backups
- **Logs**: 30-day retention

### Backup Verification

```bash
# Weekly backup test
./scripts/test-backup-restore.sh

# Verify backup integrity
./scripts/verify-backups.sh
```

### Recovery Procedures

1. **Database Recovery**
   ```bash
   # Identify backup point
   # Restore from backup
   # Verify data integrity
   # Resume operations
   ```

2. **Full System Recovery**
   - Restore infrastructure
   - Restore database
   - Restore configuration
   - Verify functionality

---

## Scaling Procedures

### Auto-Scaling

**CPU-Based Scaling**
- Scale up: CPU > 70% for 5 min
- Scale down: CPU < 30% for 10 min

**Memory-Based Scaling**
- Scale up: Memory > 80% for 5 min
- Scale down: Memory < 40% for 10 min

### Manual Scaling

```bash
# Scale up
./scripts/scale-up.sh --instances 5

# Scale down
./scripts/scale-down.sh --instances 2

# Verify scaling
./scripts/check-scaling.sh
```

---

## Maintenance Windows

### Scheduled Maintenance

- **Frequency**: Monthly (first Sunday, 2-4 AM UTC)
- **Duration**: 2 hours
- **Notification**: 1 week advance notice

### Maintenance Checklist

- [ ] Notify users
- [ ] Create maintenance ticket
- [ ] Backup all systems
- [ ] Perform maintenance
- [ ] Verify functionality
- [ ] Close maintenance window
- [ ] Post-maintenance report

---

## Emergency Contacts

- **On-Call Engineer**: See PagerDuty
- **Engineering Lead**: [Contact Info]
- **Infrastructure Team**: [Contact Info]
- **Security Team**: [Contact Info]

---

## Resources

- [Incident Response Guide](./incident-response.md)
- [Launch Checklist](./launch-checklist.md)
- [Architecture Documentation](../docs/architecture.md)
- [Status Page](https://status.mockforge.dev)

---

**Last Updated:** 2025-01-27
**Review Frequency:** Monthly
