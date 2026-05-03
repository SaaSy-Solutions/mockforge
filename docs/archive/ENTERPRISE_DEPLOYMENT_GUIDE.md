# Enterprise Deployment Guide

Comprehensive guide for deploying MockForge in enterprise environments with high availability, disaster recovery, and robust backup/restore capabilities.

## Table of Contents

- [Overview](#overview)
- [High Availability (HA)](#high-availability-ha)
- [Disaster Recovery (DR)](#disaster-recovery-dr)
- [Backup and Restore](#backup-and-restore)
- [Multi-Region Deployment](#multi-region-deployment)
- [Monitoring and Alerting](#monitoring-and-alerting)
- [Security Hardening](#security-hardening)
- [Performance Optimization](#performance-optimization)
- [Platform-Specific Configurations](#platform-specific-configurations)
- [Best Practices](#best-practices)

---

## Overview

This guide provides enterprise-grade deployment patterns for MockForge, ensuring:

- **99.9%+ Uptime**: High availability with automatic failover
- **RTO < 1 hour**: Recovery Time Objective under 1 hour
- **RPO < 15 minutes**: Recovery Point Objective under 15 minutes
- **Zero Data Loss**: Comprehensive backup and restore strategies
- **Global Scale**: Multi-region deployment capabilities
- **Security Compliance**: Enterprise security standards

### Architecture Principles

1. **Stateless Application Layer**: MockForge instances are stateless and horizontally scalable
2. **Shared State Storage**: Use external databases and caches for shared state
3. **Health Checks**: Comprehensive health monitoring at multiple levels
4. **Graceful Degradation**: System continues operating with reduced functionality during failures
5. **Automated Recovery**: Self-healing capabilities with minimal manual intervention

---

## High Availability (HA)

### Architecture Pattern

```
                    ┌─────────────────┐
                    │  Load Balancer  │
                    │   (HA Proxy)    │
                    └────────┬────────┘
                             │
                ┌────────────┼────────────┐
                │            │            │
        ┌───────▼────┐ ┌────▼────┐ ┌────▼────┐
        │ MockForge  │ │MockForge│ │MockForge│
        │ Instance 1 │ │Instance 2│ │Instance 3│
        └───────┬────┘ └────┬────┘ └────┬────┘
                │            │            │
                └────────────┼────────────┘
                             │
                ┌────────────┼────────────┐
                │            │            │
        ┌───────▼────┐ ┌────▼────┐ ┌────▼────┐
        │ PostgreSQL │ │  Redis  │ │   S3     │
        │  (Primary) │ │ (Cache) │ │(Storage) │
        └───────┬────┘ └─────────┘ └──────────┘
                │
        ┌───────▼────┐
        │ PostgreSQL │
        │ (Replica)  │
        └────────────┘
```

### Component Requirements

#### 1. Application Layer (MockForge Instances)

**Minimum Configuration:**
- **3+ instances** across different availability zones
- **Health checks** every 5 seconds
- **Graceful shutdown** with 30-second drain period
- **Resource limits**: 2 CPU, 4GB RAM per instance

**Kubernetes Deployment Example:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mockforge
  namespace: mockforge
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0  # Zero-downtime updates
  selector:
    matchLabels:
      app: mockforge
  template:
    metadata:
      labels:
        app: mockforge
    spec:
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
          - labelSelector:
              matchExpressions:
              - key: app
                operator: In
                values:
                - mockforge
            topologyKey: kubernetes.io/hostname
      containers:
      - name: mockforge
        image: mockforge/mockforge:latest
        ports:
        - containerPort: 3000
          name: http
        - containerPort: 3001
          name: admin
        resources:
          requests:
            cpu: "1"
            memory: "2Gi"
          limits:
            cpu: "2"
            memory: "4Gi"
        livenessProbe:
          httpGet:
            path: /health/live
            port: admin
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: admin
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
        startupProbe:
          httpGet:
            path: /health/startup
            port: admin
          initialDelaySeconds: 0
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 30
        env:
        - name: MOCKFORGE_DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: mockforge-secrets
              key: database-url
        - name: MOCKFORGE_REDIS_URL
          valueFrom:
            secretKeyRef:
              name: mockforge-secrets
              key: redis-url
```

#### 2. Load Balancer

**Requirements:**
- **Health checks** on `/health/ready` endpoint
- **Session affinity** (sticky sessions) for WebSocket connections
- **SSL/TLS termination** with automatic certificate renewal
- **Connection draining** during deployments

**NGINX Configuration:**

```nginx
upstream mockforge {
    least_conn;
    server mockforge-1:3000 max_fails=3 fail_timeout=30s;
    server mockforge-2:3000 max_fails=3 fail_timeout=30s;
    server mockforge-3:3000 max_fails=3 fail_timeout=30s;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name mockforge.example.com;

    ssl_certificate /etc/ssl/certs/mockforge.crt;
    ssl_certificate_key /etc/ssl/private/mockforge.key;
    ssl_protocols TLSv1.2 TLSv1.3;

    location / {
        proxy_pass http://mockforge;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    location /health {
        access_log off;
        proxy_pass http://mockforge;
    }
}
```

#### 3. Database Layer

**PostgreSQL High Availability:**

```yaml
# Primary Database
apiVersion: postgresql.cnpg.io/v1
kind: Cluster
metadata:
  name: mockforge-db
spec:
  instances: 3
  postgresql:
    parameters:
      max_connections: "200"
      shared_buffers: "256MB"
      effective_cache_size: "1GB"
  backup:
    retentionPolicy: "30d"
    barmanObjectStore:
      destinationPath: "s3://backups/mockforge/db"
      s3Credentials:
        accessKeyId:
          name: backup-credentials
          key: ACCESS_KEY_ID
        secretAccessKey:
          name: backup-credentials
          key: SECRET_ACCESS_KEY
      wal:
        retention: "7d"
      data:
        retention: "30d"
```

**Connection Pooling (PgBouncer):**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pgbouncer
spec:
  replicas: 2
  template:
    spec:
      containers:
      - name: pgbouncer
        image: pgbouncer/pgbouncer:latest
        env:
        - name: DATABASES_HOST
          value: "mockforge-db-rw"
        - name: DATABASES_PORT
          value: "5432"
        - name: DATABASES_DBNAME
          value: "mockforge"
        - name: POOL_MODE
          value: "transaction"
        - name: MAX_CLIENT_CONN
          value: "1000"
        - name: DEFAULT_POOL_SIZE
          value: "25"
```

#### 4. Cache Layer (Redis)

**Redis Sentinel for HA:**

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: redis
spec:
  serviceName: redis
  replicas: 3
  template:
    spec:
      containers:
      - name: redis
        image: redis:7-alpine
        command:
        - redis-server
        - /etc/redis/redis.conf
        - --sentinel
        - --sentinel-announce-ip $(POD_IP)
        ports:
        - containerPort: 6379
          name: redis
        - containerPort: 26379
          name: sentinel
        volumeMounts:
        - name: config
          mountPath: /etc/redis
        - name: data
          mountPath: /data
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 10Gi
```

### Health Check Strategy

**Multi-Level Health Checks:**

1. **Liveness Probe**: Detects if the process is running
   - Endpoint: `/health/live`
   - Interval: 5 seconds
   - Timeout: 3 seconds
   - Failure threshold: 3 (restart after 15 seconds)

2. **Readiness Probe**: Detects if the instance can accept traffic
   - Endpoint: `/health/ready`
   - Checks: Database connectivity, Redis connectivity, disk space
   - Interval: 5 seconds
   - Timeout: 3 seconds
   - Failure threshold: 2 (remove from load balancer after 10 seconds)

3. **Startup Probe**: Allows slow-starting instances time to initialize
   - Endpoint: `/health/startup`
   - Interval: 5 seconds
   - Timeout: 3 seconds
   - Failure threshold: 30 (2.5 minutes startup time)

**Custom Health Check Implementation:**

```rust
// Example health check endpoint
async fn health_ready() -> Result<Json<HealthStatus>, StatusCode> {
    let mut status = HealthStatus {
        status: "healthy".to_string(),
        checks: HashMap::new(),
    };

    // Check database
    match check_database().await {
        Ok(_) => {
            status.checks.insert("database".to_string(), "ok".to_string());
        }
        Err(e) => {
            status.status = "unhealthy".to_string();
            status.checks.insert("database".to_string(), format!("error: {}", e));
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    // Check Redis
    match check_redis().await {
        Ok(_) => {
            status.checks.insert("redis".to_string(), "ok".to_string());
        }
        Err(e) => {
            status.status = "degraded".to_string();
            status.checks.insert("redis".to_string(), format!("warning: {}", e));
            // Redis failure is non-critical, continue
        }
    }

    // Check disk space
    match check_disk_space().await {
        Ok(_) => {
            status.checks.insert("disk".to_string(), "ok".to_string());
        }
        Err(e) => {
            status.status = "unhealthy".to_string();
            status.checks.insert("disk".to_string(), format!("error: {}", e));
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    Ok(Json(status))
}
```

### Failover Scenarios

#### Scenario 1: Instance Failure

**Detection:**
- Liveness probe fails 3 times (15 seconds)
- Instance is automatically restarted by orchestrator

**Recovery:**
- New instance starts and passes startup probe
- Readiness probe passes, instance added to load balancer
- Traffic automatically routed to healthy instances during recovery

**Expected Downtime:** 0 seconds (other instances handle traffic)

#### Scenario 2: Database Primary Failure

**Detection:**
- Database connection errors in health checks
- Monitoring alerts trigger

**Recovery:**
- PostgreSQL automatic failover to replica (30-60 seconds)
- Application instances reconnect to new primary
- Readiness probes pass, traffic resumes

**Expected Downtime:** 30-60 seconds

#### Scenario 3: Load Balancer Failure

**Detection:**
- External monitoring detects service unavailability
- DNS health checks fail

**Recovery:**
- Secondary load balancer activated
- DNS updated to point to secondary
- Traffic resumes

**Expected Downtime:** 1-2 minutes (with automated failover)

---

## Disaster Recovery (DR)

### Recovery Objectives

- **RTO (Recovery Time Objective)**: < 1 hour
- **RPO (Recovery Point Objective)**: < 15 minutes
- **Data Loss Tolerance**: Zero (with proper backup configuration)

### DR Architecture

```
Primary Region (us-east-1)          Secondary Region (us-west-2)
┌─────────────────────────┐        ┌─────────────────────────┐
│  MockForge Instances    │        │  MockForge Instances    │
│  (Active)               │        │  (Standby)              │
│                         │        │                         │
│  ┌──────────────────┐   │        │  ┌──────────────────┐   │
│  │  PostgreSQL      │   │◄───────┤  │  PostgreSQL      │   │
│  │  (Primary)       │   │ Stream │  │  (Replica)       │   │
│  └──────────────────┘   │        │  └──────────────────┘   │
│                         │        │                         │
│  ┌──────────────────┐   │        │  ┌──────────────────┐   │
│  │  Redis           │   │        │  │  Redis           │   │
│  │  (Active)        │   │        │  │  (Standby)       │   │
│  └──────────────────┘   │        │  └──────────────────┘   │
│                         │        │                         │
│  ┌──────────────────┐   │        │  ┌──────────────────┐   │
│  │  S3 Bucket       │   │◄───────┤  │  S3 Bucket       │   │
│  │  (Primary)       │   │ Replicate│  │  (Replica)      │   │
│  └──────────────────┘   │        │  └──────────────────┘   │
└─────────────────────────┘        └─────────────────────────┘
```

### Multi-Region Setup

#### 1. Database Replication

**PostgreSQL Streaming Replication:**

```sql
-- On primary database
CREATE USER replicator WITH REPLICATION PASSWORD 'secure_password';
ALTER SYSTEM SET wal_level = 'replica';
ALTER SYSTEM SET max_wal_senders = 3;
ALTER SYSTEM SET max_replication_slots = 3;

-- On replica database
-- Create recovery.conf or use postgresql.conf
primary_conninfo = 'host=primary-db.example.com port=5432 user=replicator password=secure_password'
primary_slot_name = 'mockforge_replica_slot'
```

**Automated Failover with Patroni:**

```yaml
scope: mockforge
namespace: /mockforge/
name: mockforge-db

restapi:
  listen: 0.0.0.0:8008
  connect_address: ${PATRONI_SCOPE}-${PATRONI_NAME}.${PATRONI_NAMESPACE}:8008

etcd:
  hosts: etcd-1:2379,etcd-2:2379,etcd-3:2379

bootstrap:
  dcs:
    ttl: 30
    loop_wait: 10
    retry_timeout: 30
    maximum_lag_on_failover: 1048576
    postgresql:
      use_pg_rewind: true
      parameters:
        wal_level: replica
        hot_standby: "on"
        max_connections: 200
        max_replication_slots: 3
        max_wal_senders: 3

postgresql:
  listen: 0.0.0.0:5432
  connect_address: ${PATRONI_SCOPE}-${PATRONI_NAME}.${PATRONI_NAMESPACE}:5432
  data_dir: /var/lib/postgresql/data
  pgpass: /tmp/pgpass
  authentication:
    replication:
      username: replicator
      password: secure_password
    superuser:
      username: postgres
      password: secure_password
  parameters:
    unix_socket_directories: '/var/run/postgresql'
```

#### 2. Application Deployment

**Multi-Region Kubernetes:**

```yaml
# Primary region deployment
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: mockforge-primary
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/your-org/mockforge-config
    targetRevision: main
    path: k8s/primary
  destination:
    server: https://kubernetes-primary.example.com
    namespace: mockforge
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
    - CreateNamespace=true

---
# Secondary region deployment
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: mockforge-secondary
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/your-org/mockforge-config
    targetRevision: main
    path: k8s/secondary
  destination:
    server: https://kubernetes-secondary.example.com
    namespace: mockforge
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
    - CreateNamespace=true
```

#### 3. DNS Failover

**Route53 Health Checks and Failover:**

```yaml
# Primary record
apiVersion: route53.aws.crossplane.io/v1alpha1
kind: Record
metadata:
  name: mockforge-primary
spec:
  forProvider:
    zoneId: Z1234567890ABC
    name: mockforge.example.com
    type: A
    alias:
      evaluateTargetHealth: true
      dnsName: primary-lb-123456789.us-east-1.elb.amazonaws.com
      hostedZoneId: Z35SXDOTRQ7X7K
    setIdentifier: primary
    failover: PRIMARY
    healthCheckId: abc123def456

---
# Secondary record
apiVersion: route53.aws.crossplane.io/v1alpha1
kind: Record
metadata:
  name: mockforge-secondary
spec:
  forProvider:
    zoneId: Z1234567890ABC
    name: mockforge.example.com
    type: A
    alias:
      evaluateTargetHealth: true
      dnsName: secondary-lb-987654321.us-west-2.elb.amazonaws.com
      hostedZoneId: Z1D633PJN98FT9
    setIdentifier: secondary
    failover: SECONDARY
```

### DR Runbook

#### Failover Procedure

1. **Detection** (Automated)
   - Health checks fail in primary region
   - Monitoring alerts trigger
   - DR coordinator notified

2. **Verification** (Manual)
   - Confirm primary region is unavailable
   - Check secondary region health
   - Verify database replication lag < RPO

3. **Failover Execution** (Automated/Manual)
   ```bash
   # Promote secondary database to primary
   kubectl exec -it postgres-0 -n mockforge -- \
     patronictl failover mockforge-db --candidate postgres-1

   # Update application configuration
   kubectl set env deployment/mockforge \
     -n mockforge \
     MOCKFORGE_DATABASE_URL=postgresql://secondary-db:5432/mockforge

   # Scale up secondary region instances
   kubectl scale deployment mockforge \
     -n mockforge \
     --replicas=5

   # Update DNS to point to secondary region
   aws route53 change-resource-record-sets \
     --hosted-zone-id Z1234567890ABC \
     --change-batch file://failover-dns.json
   ```

4. **Verification** (Automated)
   - Health checks pass in secondary region
   - Traffic routing confirmed
   - Application functionality verified

5. **Communication** (Manual)
   - Notify stakeholders
   - Update status page
   - Document incident

#### Failback Procedure

1. **Primary Region Recovery**
   - Restore primary region infrastructure
   - Verify database replication from secondary
   - Test application functionality

2. **Gradual Traffic Migration**
   ```bash
   # Update DNS with weighted routing (10% primary, 90% secondary)
   aws route53 change-resource-record-sets \
     --hosted-zone-id Z1234567890ABC \
     --change-batch file://weighted-routing-10-90.json

   # Monitor for 1 hour, then 50-50
   # Monitor for 1 hour, then 90-10
   # Monitor for 1 hour, then 100% primary
   ```

3. **Database Promotion**
   ```bash
   # Promote primary database back to primary role
   kubectl exec -it postgres-0 -n mockforge -- \
     patronictl failover mockforge-db --candidate postgres-0
   ```

4. **Cleanup**
   - Scale down secondary region instances
   - Update monitoring and alerting
   - Document lessons learned

---

## Backup and Restore

### Backup Strategy

#### 1. Database Backups

**Automated Daily Backups:**

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: postgres-backup
  namespace: mockforge
spec:
  schedule: "0 2 * * *"  # 2 AM daily
  successfulJobsHistoryLimit: 7
  failedJobsHistoryLimit: 3
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: backup
            image: postgres:15-alpine
            env:
            - name: PGHOST
              value: mockforge-db
            - name: PGDATABASE
              value: mockforge
            - name: PGPASSWORD
              valueFrom:
                secretKeyRef:
                  name: postgres-credentials
                  key: password
            - name: S3_BUCKET
              value: s3://backups/mockforge/db
            - name: AWS_ACCESS_KEY_ID
              valueFrom:
                secretKeyRef:
                  name: aws-credentials
                  key: access-key-id
            - name: AWS_SECRET_ACCESS_KEY
              valueFrom:
                secretKeyRef:
                  name: aws-credentials
                  key: secret-access-key
            command:
            - /bin/sh
            - -c
            - |
              BACKUP_FILE="mockforge-$(date +%Y%m%d-%H%M%S).sql.gz"
              pg_dump -Fc $PGDATABASE | gzip | \
              aws s3 cp - s3://$S3_BUCKET/$BACKUP_FILE

              # Keep only last 30 days
              aws s3 ls s3://$S3_BUCKET/ | \
              awk '{print $4}' | \
              sort -r | \
              tail -n +31 | \
              xargs -I {} aws s3 rm s3://$S3_BUCKET/{}
          restartPolicy: OnFailure
```

**Point-in-Time Recovery (PITR):**

```yaml
# Continuous WAL archiving
apiVersion: v1
kind: ConfigMap
metadata:
  name: postgres-config
  namespace: mockforge
data:
  postgresql.conf: |
    wal_level = replica
    archive_mode = on
    archive_command = 'aws s3 cp %p s3://backups/mockforge/wal/%f'
    max_wal_senders = 3
    wal_keep_size = 1GB
```

#### 2. Application Configuration Backups

**Git-based Configuration Management:**

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: config-backup
  namespace: mockforge
data:
  backup.sh: |
    #!/bin/bash
    # Backup all configuration files
    kubectl get configmap -n mockforge -o yaml > config-backup-$(date +%Y%m%d).yaml
    kubectl get secret -n mockforge -o yaml > secrets-backup-$(date +%Y%m%d).yaml

    # Encrypt and upload to S3
    tar czf - config-backup-*.yaml secrets-backup-*.yaml | \
    gpg --encrypt --recipient backup@example.com | \
    aws s3 cp - s3://backups/mockforge/config/config-$(date +%Y%m%d).tar.gz.gpg
```

#### 3. State and Data Backups

**Redis Backup:**

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: redis-backup
  namespace: mockforge
spec:
  schedule: "0 */6 * * *"  # Every 6 hours
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: backup
            image: redis:7-alpine
            command:
            - /bin/sh
            - -c
            - |
              redis-cli -h redis-master --rdb /backup/dump.rdb
              aws s3 cp /backup/dump.rdb \
                s3://backups/mockforge/redis/redis-$(date +%Y%m%d-%H%M%S).rdb
            volumeMounts:
            - name: backup
              mountPath: /backup
          volumes:
          - name: backup
            emptyDir: {}
```

### Restore Procedures

#### 1. Database Restore

**Full Database Restore:**

```bash
# Download backup
aws s3 cp s3://backups/mockforge/db/mockforge-20240101-020000.sql.gz \
  /tmp/backup.sql.gz

# Restore to new database
gunzip -c /tmp/backup.sql.gz | \
  psql -h new-db-host -U postgres -d mockforge

# Verify restore
psql -h new-db-host -U postgres -d mockforge -c \
  "SELECT COUNT(*) FROM admin_users;"
```

**Point-in-Time Recovery:**

```bash
# Restore base backup
pg_basebackup -h backup-server -D /var/lib/postgresql/data -P -W

# Restore WAL files up to target time
recovery_target_time = '2024-01-01 14:30:00'
recovery_target_action = 'promote'

# Start PostgreSQL
pg_ctl start -D /var/lib/postgresql/data
```

#### 2. Application Configuration Restore

```bash
# Download encrypted backup
aws s3 cp s3://backups/mockforge/config/config-20240101.tar.gz.gpg \
  /tmp/config-backup.tar.gz.gpg

# Decrypt and extract
gpg --decrypt /tmp/config-backup.tar.gz.gpg | tar xzf -

# Restore ConfigMaps and Secrets
kubectl apply -f config-backup-20240101.yaml
kubectl apply -f secrets-backup-20240101.yaml
```

#### 3. Complete System Restore

**Automated Restore Script:**

```bash
#!/bin/bash
set -e

BACKUP_DATE=${1:-$(date +%Y%m%d)}
NAMESPACE="mockforge"

echo "Starting restore from backup: $BACKUP_DATE"

# 1. Restore database
echo "Restoring database..."
aws s3 cp s3://backups/mockforge/db/mockforge-${BACKUP_DATE}-020000.sql.gz \
  /tmp/db-backup.sql.gz
gunzip -c /tmp/db-backup.sql.gz | \
  psql -h $DB_HOST -U $DB_USER -d $DB_NAME

# 2. Restore Redis
echo "Restoring Redis..."
aws s3 cp s3://backups/mockforge/redis/redis-${BACKUP_DATE}-020000.rdb \
  /tmp/redis-backup.rdb
kubectl cp /tmp/redis-backup.rdb \
  ${NAMESPACE}/redis-0:/data/dump.rdb
kubectl exec -n ${NAMESPACE} redis-0 -- \
  redis-cli DEBUG RELOAD

# 3. Restore configuration
echo "Restoring configuration..."
aws s3 cp s3://backups/mockforge/config/config-${BACKUP_DATE}.tar.gz.gpg \
  /tmp/config-backup.tar.gz.gpg
gpg --decrypt /tmp/config-backup.tar.gz.gpg | tar xzf -
kubectl apply -f config-backup-${BACKUP_DATE}.yaml
kubectl apply -f secrets-backup-${BACKUP_DATE}.yaml

# 4. Restart applications
echo "Restarting applications..."
kubectl rollout restart deployment/mockforge -n ${NAMESPACE}

# 5. Verify restore
echo "Verifying restore..."
kubectl wait --for=condition=available \
  --timeout=300s \
  deployment/mockforge -n ${NAMESPACE}

echo "Restore completed successfully!"
```

### Backup Verification

**Automated Backup Testing:**

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: backup-verification
  namespace: mockforge
spec:
  schedule: "0 3 * * 0"  # Weekly on Sunday at 3 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: verify
            image: postgres:15-alpine
            command:
            - /bin/sh
            - -c
            - |
              # Download latest backup
              LATEST_BACKUP=$(aws s3 ls s3://backups/mockforge/db/ | \
                sort -r | head -1 | awk '{print $4}')
              aws s3 cp s3://backups/mockforge/db/$LATEST_BACKUP /tmp/backup.sql.gz

              # Restore to test database
              gunzip -c /tmp/backup.sql.gz | \
                psql -h test-db -U postgres -d mockforge_test

              # Verify data integrity
              RECORD_COUNT=$(psql -h test-db -U postgres -d mockforge_test -t -c \
                "SELECT COUNT(*) FROM admin_users;")

              if [ "$RECORD_COUNT" -gt "0" ]; then
                echo "Backup verification successful: $RECORD_COUNT records"
                exit 0
              else
                echo "Backup verification failed: No records found"
                exit 1
              fi
```

---

## Monitoring and Alerting

### Key Metrics

**Application Metrics:**
- Request rate (requests/second)
- Error rate (errors/second)
- Latency (p50, p95, p99)
- Active connections
- Memory usage
- CPU usage

**Infrastructure Metrics:**
- Database connection pool usage
- Database replication lag
- Redis memory usage
- Disk I/O
- Network throughput

**Business Metrics:**
- Active mock servers
- API calls per mock
- User sessions
- Feature usage

### Alerting Rules

**Critical Alerts (Page Immediately):**

```yaml
groups:
- name: critical
  interval: 30s
  rules:
  - alert: MockForgeDown
    expr: up{job="mockforge"} == 0
    for: 1m
    annotations:
      summary: "MockForge instance is down"
      description: "Instance {{ $labels.instance }} has been down for more than 1 minute"

  - alert: DatabaseDown
    expr: pg_up == 0
    for: 1m
    annotations:
      summary: "Database is down"
      description: "PostgreSQL database is unreachable"

  - alert: HighErrorRate
    expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
    for: 5m
    annotations:
      summary: "High error rate detected"
      description: "Error rate is {{ $value }} errors/second"

  - alert: DatabaseReplicationLag
    expr: pg_replication_lag > 60
    for: 5m
    annotations:
      summary: "Database replication lag is high"
      description: "Replication lag is {{ $value }} seconds"
```

**Warning Alerts (Notify Team):**

```yaml
- name: warning
  interval: 1m
  rules:
  - alert: HighLatency
    expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1
    for: 10m
    annotations:
      summary: "High latency detected"
      description: "95th percentile latency is {{ $value }} seconds"

  - alert: HighMemoryUsage
    expr: (container_memory_usage_bytes / container_spec_memory_limit_bytes) > 0.9
    for: 10m
    annotations:
      summary: "High memory usage"
      description: "Memory usage is {{ $value | humanizePercentage }}"

  - alert: DiskSpaceLow
    expr: (node_filesystem_avail_bytes / node_filesystem_size_bytes) < 0.1
    for: 5m
    annotations:
      summary: "Disk space is low"
      description: "Only {{ $value | humanizePercentage }} disk space remaining"
```

---

## Security Hardening

### Network Security

- **VPC/Private Networking**: Deploy in private subnets
- **Security Groups**: Restrict access to necessary ports only
- **WAF**: Web Application Firewall for DDoS protection
- **VPN/PrivateLink**: Secure access to internal services

### Data Security

- **Encryption at Rest**: Enable encryption for all databases and storage
- **Encryption in Transit**: TLS 1.2+ for all connections
- **Secrets Management**: Use Vault, AWS Secrets Manager, or similar
- **Key Rotation**: Automate key rotation for certificates and secrets

### Access Control

- **RBAC**: Role-based access control for all services
- **MFA**: Multi-factor authentication for admin access
- **Audit Logging**: Comprehensive audit logs for all actions
- **Least Privilege**: Minimum necessary permissions

---

## Performance Optimization

### Caching Strategy

- **Application Cache**: Redis for frequently accessed data
- **CDN**: CloudFront/CloudFlare for static assets
- **Database Query Cache**: Enable PostgreSQL query cache
- **Connection Pooling**: PgBouncer for database connections

### Resource Sizing

**Small Deployment (< 1000 req/s):**
- 3 instances: 1 CPU, 2GB RAM each
- Database: db.t3.medium (2 vCPU, 4GB RAM)
- Redis: cache.t3.micro (1 vCPU, 0.5GB RAM)

**Medium Deployment (1000-10000 req/s):**
- 5 instances: 2 CPU, 4GB RAM each
- Database: db.r5.large (2 vCPU, 16GB RAM)
- Redis: cache.r5.large (2 vCPU, 13GB RAM)

**Large Deployment (> 10000 req/s):**
- 10+ instances: 4 CPU, 8GB RAM each
- Database: db.r5.xlarge (4 vCPU, 32GB RAM) with read replicas
- Redis: cache.r5.xlarge (4 vCPU, 26GB RAM) cluster

---

## Platform-Specific Configurations

### AWS

See [AWS Deployment Guide](deployment/aws.md) for:
- EKS setup with multi-AZ
- RDS Multi-AZ with read replicas
- ElastiCache Redis cluster
- S3 cross-region replication
- Route53 health checks and failover

### Google Cloud Platform

See [GCP Deployment Guide](deployment/gcp.md) for:
- GKE regional clusters
- Cloud SQL high availability
- Cloud Memorystore Redis
- Cloud Storage multi-region buckets
- Cloud Load Balancing with health checks

### Azure

See [Azure Deployment Guide](deployment/azure.md) for:
- ASK availability zones
- Azure Database for PostgreSQL with read replicas
- Azure Cache for Redis premium
- Azure Storage geo-redundant
- Azure Traffic Manager

---

## Best Practices

1. **Start with HA, Add DR Later**: Implement high availability first, then add disaster recovery
2. **Test Regularly**: Run DR drills quarterly
3. **Automate Everything**: Minimize manual intervention
4. **Monitor Continuously**: 24/7 monitoring with on-call rotation
5. **Document Thoroughly**: Keep runbooks up to date
6. **Version Control**: All infrastructure as code
7. **Security First**: Implement security from the start
8. **Cost Optimization**: Right-size resources, use reserved instances
9. **Backup Verification**: Regularly test backup restores
10. **Incident Response**: Have clear incident response procedures

---

## Additional Resources

- [Kubernetes Deployment Guide](CLOUD_DEPLOYMENT.md#kubernetes-deployment)
- [Docker Deployment Guide](../DOCKER.md)
- [Helm Chart Documentation](../helm/mockforge/README.md)
- [Monitoring Setup Guide](ADVANCED_OBSERVABILITY.md)
- [Security Whitepaper](SECURITY_WHITEPAPER.md)

---

**Last Updated**: 2024-01-01
**Version**: 1.0
