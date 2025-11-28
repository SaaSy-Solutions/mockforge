# MockForge Managed Hosting Guide

Comprehensive guide for deploying and managing MockForge in production environments with focus on scaling, high availability, and multi-region deployment.

## Table of Contents

- [Overview](#overview)
- [Architecture Patterns](#architecture-patterns)
- [Scaling Strategies](#scaling-strategies)
- [Multi-Region Deployment](#multi-region-deployment)
- [High Availability](#high-availability)
- [State Management](#state-management)
- [Load Balancing](#load-balancing)
- [Monitoring & Observability](#monitoring--observability)
- [Cost Optimization](#cost-optimization)
- [Security Considerations](#security-considerations)
- [Disaster Recovery](#disaster-recovery)
- [Platform-Specific Guides](#platform-specific-guides)

---

## Overview

MockForge can be deployed as a managed service with enterprise-grade features including:

- **Auto-scaling**: Automatically scale based on traffic
- **High Availability**: Multi-instance deployments with failover
- **Multi-Region**: Deploy across multiple geographic regions
- **State Synchronization**: Shared state across instances
- **Load Balancing**: Distribute traffic across instances
- **Monitoring**: Comprehensive observability stack

### Deployment Models

1. **Serverless** (Cloud Run, Lambda, App Engine)
   - Pay-per-use pricing
   - Automatic scaling
   - Zero infrastructure management
   - Best for: Variable traffic, cost optimization

2. **Container Orchestration** (Kubernetes, ECS, ASK)
   - Full control over scaling
   - Advanced networking
   - Best for: High traffic, complex requirements

3. **Managed Services** (App Platform, Elastic Beanstalk)
   - Simplified deployment
   - Built-in scaling
   - Best for: Quick deployment, moderate traffic

---

## Architecture Patterns

### Pattern 1: Stateless Horizontal Scaling

**Best for**: High throughput, simple deployments

```
                    ┌─────────────┐
                    │ Load Balancer│
                    └──────┬──────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐       ┌────▼────┐       ┌────▼────┐
   │Instance │       │Instance │       │Instance │
   │    1    │       │    2    │       │    3    │
   └─────────┘       └─────────┘       └─────────┘
        │                  │                  │
        └──────────────────┼──────────────────┘
                           │
                    ┌──────▼──────┐
                    │Shared State │
                    │  (Redis)    │
                    └─────────────┘
```

**Configuration**:
- All instances are stateless
- Shared state in Redis/PostgreSQL
- Session affinity disabled
- Health checks enabled

### Pattern 2: Multi-Region Active-Active

**Best for**: Global deployments, low latency

```
Region 1 (US-East)          Region 2 (EU-West)          Region 3 (AP-South)
┌──────────────┐            ┌──────────────┐            ┌──────────────┐
│ Load Balancer│            │ Load Balancer│            │ Load Balancer│
└──────┬───────┘            └──────┬───────┘            └──────┬───────┘
       │                          │                          │
  ┌────▼────┐                ┌────▼────┐                ┌────▼────┐
  │Instance │                │Instance │                │Instance │
  │   1-3   │                │   1-3   │                │   1-3   │
  └────┬────┘                └────┬────┘                └────┬────┘
       │                          │                          │
       └──────────────┬───────────┴───────────┬──────────────┘
                       │                       │
              ┌────────▼────────┐    ┌─────────▼─────────┐
              │ Global Database │    │  State Sync Layer │
              │   (PostgreSQL)  │    │    (Redis)        │
              └─────────────────┘    └───────────────────┘
```

**Configuration**:
- Each region has independent instances
- Global database for shared state
- GeoDNS for routing
- Cross-region replication

### Pattern 3: Hybrid (Stateless + Stateful)

**Best for**: Complex scenarios with stateful features

```
                    ┌─────────────┐
                    │ Load Balancer│
                    └──────┬──────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐       ┌────▼────┐       ┌────▼────┐
   │Stateless │       │Stateless │       │Stateless │
   │Instance │       │Instance │       │Instance │
   └─────────┘       └─────────┘       └─────────┘
        │                  │                  │
        └──────────────────┼──────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐       ┌────▼────┐       ┌────▼────┐
   │Stateful │       │Stateful │       │Stateful │
   │Instance │       │Instance │       │Instance │
   └─────────┘       └─────────┘       └─────────┘
```

**Configuration**:
- Stateless instances handle HTTP/WS traffic
- Stateful instances handle stateful features
- Separate scaling policies

---

## Scaling Strategies

### Auto-Scaling Configuration

#### Kubernetes (HPA)

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: mockforge-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: mockforge
  minReplicas: 2
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0
      policies:
      - type: Percent
        value: 100
        periodSeconds: 15
      - type: Pods
        value: 4
        periodSeconds: 15
      selectPolicy: Max
```

#### Cloud Run (Serverless)

```yaml
# cloud-run-config.yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: mockforge
spec:
  template:
    metadata:
      annotations:
        autoscaling.knative.dev/minScale: "2"
        autoscaling.knative.dev/maxScale: "100"
        autoscaling.knative.dev/target: "80"
    spec:
      containerConcurrency: 80
      containers:
      - image: ghcr.io/saasy-solutions/mockforge:latest
        resources:
          limits:
            cpu: "2"
            memory: 2Gi
          requests:
            cpu: "1"
            memory: 1Gi
```

#### AWS ECS (Fargate)

```json
{
  "serviceName": "mockforge",
  "desiredCount": 2,
  "launchType": "FARGATE",
  "networkConfiguration": {
    "awsvpcConfiguration": {
      "subnets": ["subnet-xxx"],
      "assignPublicIp": "ENABLED"
    }
  },
  "autoScalingConfiguration": {
    "minCapacity": 2,
    "maxCapacity": 50,
    "targetTrackingScalingPolicies": [
      {
        "targetValue": 70.0,
        "predefinedMetricSpecification": {
          "predefinedMetricType": "ECSServiceAverageCPUUtilization"
        }
      },
      {
        "targetValue": 80.0,
        "predefinedMetricSpecification": {
          "predefinedMetricType": "ECSServiceAverageMemoryUtilization"
        }
      }
    ]
  }
}
```

### Scaling Metrics

**CPU-Based Scaling**:
- Target: 70% CPU utilization
- Scale up when > 80% for 2 minutes
- Scale down when < 50% for 5 minutes

**Memory-Based Scaling**:
- Target: 80% memory utilization
- Scale up when > 90% for 1 minute
- Scale down when < 60% for 5 minutes

**Request-Based Scaling**:
- Scale up when requests/instance > 1000/min
- Scale down when requests/instance < 200/min

**Custom Metrics**:
- Active WebSocket connections
- Queue depth
- Response time (p95 > 500ms)

### Scaling Policies

**Conservative** (Cost-Optimized):
- Slow scale-up (5 minutes)
- Fast scale-down (2 minutes)
- Lower thresholds (60% CPU)

**Aggressive** (Performance-Optimized):
- Fast scale-up (30 seconds)
- Slow scale-down (10 minutes)
- Higher thresholds (80% CPU)

**Balanced** (Recommended):
- Moderate scale-up (2 minutes)
- Moderate scale-down (5 minutes)
- Medium thresholds (70% CPU)

---

## Multi-Region Deployment

### Architecture

```
                    ┌─────────────────┐
                    │   GeoDNS/CDN    │
                    │  (Route 53)     │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
   ┌────▼────┐         ┌────▼────┐         ┌────▼────┐
   │ US-East │         │ EU-West │         │ AP-South│
   │ Region  │         │ Region  │         │ Region  │
   └────┬────┘         └────┬────┘         └────┬────┘
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
                    ┌────────▼────────┐
                    │ Global Database │
                    │  (PostgreSQL)   │
                    └─────────────────┘
```

### Configuration

#### Region-Specific Settings

```yaml
# config.us-east.yaml
server:
  region: us-east-1
  http_port: 3000
  admin_port: 9080

database:
  host: global-db.us-east-1.rds.amazonaws.com
  region: us-east-1

redis:
  host: redis-cluster.us-east-1.cache.amazonaws.com
  region: us-east-1

monitoring:
  region: us-east-1
  endpoint: https://monitoring.us-east-1.amazonaws.com
```

#### GeoDNS Configuration (Route 53)

```yaml
# route53-config.yaml
records:
  - name: api.mockforge.dev
    type: A
    alias:
      evaluate_target_health: true
      dns_name: d1234abc.cloudfront.net
    geolocation_routing:
      - continent_code: NA
        region: us-east-1
        endpoint: us-east-1.elb.amazonaws.com
      - continent_code: EU
        region: eu-west-1
        endpoint: eu-west-1.elb.amazonaws.com
      - continent_code: AS
        region: ap-south-1
        endpoint: ap-south-1.elb.amazonaws.com
```

### State Synchronization

#### Cross-Region Replication

```yaml
# State sync configuration
state_sync:
  enabled: true
  method: redis_streams  # or postgres_logical_replication
  regions:
    - us-east-1
    - eu-west-1
    - ap-south-1
  replication_lag_threshold: 5s
  conflict_resolution: last_write_wins
```

#### Conflict Resolution

1. **Last Write Wins** (Default):
   - Timestamp-based resolution
   - Simple and fast
   - May lose some updates

2. **Vector Clocks**:
   - Causal ordering
   - More complex
   - Preserves causality

3. **CRDTs** (Conflict-free Replicated Data Types):
   - Automatic merging
   - Complex implementation
   - No conflicts

### Latency Optimization

**Edge Caching**:
- Static assets via CDN
- Cache-Control headers
- TTL: 1 hour for configs, 24 hours for assets

**Database Read Replicas**:
- Read from local replica
- Write to primary (async replication)

**Connection Pooling**:
- Per-region connection pools
- Keep-alive connections
- Connection limits per instance

---

## High Availability

### Redundancy Requirements

**Minimum Configuration**:
- 2+ instances per region
- 2+ availability zones
- Multi-region database
- Health checks enabled

**Production Configuration**:
- 3+ instances per region
- 3+ availability zones
- Multi-region database with replicas
- Automated failover
- Circuit breakers

### Health Checks

```yaml
# Health check configuration
health:
  liveness:
    path: /health/live
    interval: 30s
    timeout: 5s
    failure_threshold: 3
    success_threshold: 1

  readiness:
    path: /health/ready
    interval: 10s
    timeout: 3s
    failure_threshold: 3
    success_threshold: 1

  startup:
    path: /health/startup
    interval: 5s
    timeout: 2s
    failure_threshold: 30
    success_threshold: 1
```

### Circuit Breakers

```yaml
# Circuit breaker configuration
circuit_breaker:
  enabled: true
  failure_threshold: 5
  success_threshold: 2
  timeout: 60s
  half_open_requests: 3

  targets:
    - name: database
      failure_threshold: 10
      timeout: 30s
    - name: redis
      failure_threshold: 5
      timeout: 15s
    - name: external_api
      failure_threshold: 3
      timeout: 10s
```

### Failover Strategies

**Automatic Failover**:
- Database: Automatic promotion of replica
- Load Balancer: Remove unhealthy instances
- DNS: Failover to secondary region

**Manual Failover**:
- Graceful shutdown of primary
- Promote secondary to primary
- Update DNS/load balancer

---

## State Management

### Stateless Configuration

```yaml
# Stateless mode
state:
  mode: stateless
  storage: external  # Redis or PostgreSQL

redis:
  host: redis-cluster.example.com
  port: 6379
  db: 0
  password: ${REDIS_PASSWORD}
  cluster_mode: true
  nodes:
    - redis-1.example.com:6379
    - redis-2.example.com:6379
    - redis-3.example.com:6379
```

### Stateful Configuration

```yaml
# Stateful mode with persistence
state:
  mode: stateful
  storage: local  # Local filesystem
  persistence:
    enabled: true
    path: /data/state
    backup_interval: 1h
    backup_retention: 7d

  # Sync to external storage
  sync:
    enabled: true
    target: redis
    interval: 30s
```

### Shared State Backends

**Redis Cluster**:
- High performance
- In-memory storage
- Persistence options
- Best for: Session data, caching

**PostgreSQL**:
- ACID guarantees
- Complex queries
- Best for: Persistent state, complex data

**DynamoDB** (AWS):
- Managed service
- Auto-scaling
- Global tables
- Best for: AWS deployments

---

## Load Balancing

### Load Balancer Configuration

#### Application Load Balancer (AWS)

```yaml
# ALB configuration
load_balancer:
  type: application
  scheme: internet-facing
  subnets:
    - subnet-public-1a
    - subnet-public-1b
    - subnet-public-1c

  listeners:
    - port: 443
      protocol: HTTPS
      certificates:
        - arn:aws:acm:us-east-1:xxx:certificate/xxx
      default_action:
        type: forward
        target_group: mockforge-tg

  target_groups:
    - name: mockforge-tg
      protocol: HTTP
      port: 3000
      health_check:
        path: /health/ready
        interval: 30s
        timeout: 5s
        healthy_threshold: 2
        unhealthy_threshold: 3
      stickiness:
        enabled: false  # Stateless
```

#### NGINX Ingress (Kubernetes)

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: mockforge-ingress
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/load-balance: "round_robin"
    nginx.ingress.kubernetes.io/upstream-hash-by: "$request_uri"
spec:
  tls:
  - hosts:
    - api.mockforge.dev
    secretName: mockforge-tls
  rules:
  - host: api.mockforge.dev
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: mockforge
            port:
              number: 3000
```

### Load Balancing Algorithms

**Round Robin** (Default):
- Equal distribution
- Simple
- Best for: Stateless workloads

**Least Connections**:
- Route to instance with fewest connections
- Best for: Long-lived connections (WebSocket)

**IP Hash**:
- Consistent routing by IP
- Session affinity
- Best for: Stateful workloads

**Weighted Round Robin**:
- Weighted distribution
- Best for: Mixed instance sizes

---

## Monitoring & Observability

### Metrics Collection

```yaml
# Prometheus configuration
metrics:
  enabled: true
  port: 9090
  path: /metrics

  exporters:
    - prometheus
    - statsd
    - datadog

  custom_metrics:
    - name: requests_per_second
      type: counter
    - name: response_time_p95
      type: histogram
    - name: active_connections
      type: gauge
```

### Logging

```yaml
# Structured logging
logging:
  level: info
  format: json

  outputs:
    - stdout
    - file: /var/log/mockforge/app.log
    - cloudwatch  # AWS
    - stackdriver  # GCP
    - datadog

  fields:
    - timestamp
    - level
    - message
    - request_id
    - user_id
    - region
```

### Distributed Tracing

```yaml
# OpenTelemetry configuration
tracing:
  enabled: true
  exporter: jaeger

  jaeger:
    endpoint: http://jaeger-collector:14268/api/traces
    service_name: mockforge

  sampling:
    rate: 0.1  # 10% of requests

  tags:
    - region: ${REGION}
    - environment: ${ENV}
```

### Alerting

```yaml
# Alert rules
alerts:
  - name: high_error_rate
    condition: error_rate > 0.05
    duration: 5m
    severity: critical

  - name: high_latency
    condition: p95_latency > 1000ms
    duration: 5m
    severity: warning

  - name: low_availability
    condition: availability < 0.99
    duration: 10m
    severity: critical

  - name: high_memory_usage
    condition: memory_usage > 0.90
    duration: 5m
    severity: warning
```

---

## Cost Optimization

### Resource Sizing

**Development**:
- 1 instance
- 512MB RAM
- 0.5 CPU
- Cost: ~$10-20/month

**Staging**:
- 2 instances
- 1GB RAM
- 1 CPU
- Cost: ~$40-60/month

**Production (Small)**:
- 3 instances
- 2GB RAM
- 2 CPU
- Cost: ~$120-180/month

**Production (Medium)**:
- 5 instances
- 4GB RAM
- 4 CPU
- Cost: ~$400-600/month

**Production (Large)**:
- 10+ instances
- 8GB RAM
- 8 CPU
- Cost: ~$1000+/month

### Cost Optimization Strategies

1. **Right-Sizing**:
   - Monitor actual usage
   - Adjust instance sizes
   - Use spot instances (non-production)

2. **Auto-Scaling**:
   - Scale down during low traffic
   - Scale up only when needed
   - Use predictive scaling

3. **Reserved Instances**:
   - 1-3 year commitments
   - 30-70% savings
   - Best for: Stable workloads

4. **Serverless**:
   - Pay per request
   - No idle costs
   - Best for: Variable traffic

---

## Security Considerations

### Network Security

```yaml
# Network policies
network:
  ingress:
    - from: [0.0.0.0/0]
      ports: [443, 80]
    - from: [10.0.0.0/8]  # Internal
      ports: [3000, 9080]

  egress:
    - to: [0.0.0.0/0]
      ports: [443, 80, 53]  # HTTPS, HTTP, DNS

  firewall:
    enabled: true
    rules:
      - allow: 443/tcp
      - allow: 80/tcp
      - deny: all
```

### Authentication & Authorization

```yaml
# Auth configuration
auth:
  enabled: true
  method: jwt

  jwt:
    issuer: https://auth.mockforge.dev
    audience: mockforge-api
    public_key_url: https://auth.mockforge.dev/.well-known/jwks.json

  rbac:
    enabled: true
    policies:
      - resource: /api/*
        actions: [read, write]
        roles: [admin, user]
```

### Secrets Management

```yaml
# Secrets configuration
secrets:
  backend: vault  # or aws-secrets-manager, gcp-secret-manager

  vault:
    address: https://vault.example.com
    path: secret/mockforge
    auth_method: kubernetes

  rotation:
    enabled: true
    interval: 24h
```

---

## Disaster Recovery

### Backup Strategy

```yaml
# Backup configuration
backup:
  enabled: true

  database:
    schedule: "0 2 * * *"  # Daily at 2 AM
    retention: 30d
    storage: s3://backups/mockforge/db

  state:
    schedule: "0 */6 * * *"  # Every 6 hours
    retention: 7d
    storage: s3://backups/mockforge/state

  config:
    schedule: "0 0 * * *"  # Daily
    retention: 90d
    storage: s3://backups/mockforge/config
```

### Recovery Procedures

**RTO (Recovery Time Objective)**: 1 hour
**RPO (Recovery Point Objective)**: 15 minutes

1. **Database Recovery**:
   - Restore from latest backup
   - Point-in-time recovery
   - Verify data integrity

2. **Application Recovery**:
   - Deploy from backup config
   - Restore state from backup
   - Verify functionality

3. **Full Disaster Recovery**:
   - Failover to secondary region
   - Restore from backups
   - Update DNS
   - Verify end-to-end

---

## Platform-Specific Guides

### AWS

See [AWS Deployment Guide](deployment/aws.md) for:
- ECS Fargate setup
- EKS Kubernetes deployment
- CloudFront CDN configuration
- RDS database setup
- ElastiCache Redis configuration

### Google Cloud Platform

See [GCP Deployment Guide](deployment/gcp.md) for:
- Cloud Run serverless deployment
- GKE Kubernetes setup
- Cloud SQL database
- Cloud Memorystore Redis
- Cloud CDN configuration

### Azure

See [Azure Deployment Guide](deployment/azure.md) for:
- Container Instances
- ASK Kubernetes setup
- Azure Database for PostgreSQL
- Azure Cache for Redis
- Azure Front Door CDN

### DigitalOcean

See [DigitalOcean Deployment Guide](deployment/digitalocean.md) for:
- App Platform deployment
- DOKS Kubernetes setup
- Managed Databases
- Spaces CDN

---

## Best Practices

1. **Start Small**: Begin with minimal configuration, scale as needed
2. **Monitor First**: Set up monitoring before scaling
3. **Test Failover**: Regularly test disaster recovery procedures
4. **Document Everything**: Keep deployment docs up to date
5. **Automate**: Use Infrastructure as Code (Terraform, CloudFormation)
6. **Security First**: Enable authentication, encryption, and network policies
7. **Cost Monitor**: Track costs and optimize regularly
8. **Regular Backups**: Automated backups with tested restore procedures

---

## Additional Resources

- [Docker Deployment Guide](../DOCKER.md)
- [Kubernetes Deployment Guide](CLOUD_DEPLOYMENT.md#kubernetes-deployment)
- [Helm Chart Documentation](../helm/mockforge/README.md)
- [Monitoring Guide](ADVANCED_OBSERVABILITY.md)
- [Security Guide](SECURITY_WHITEPAPER.md)
