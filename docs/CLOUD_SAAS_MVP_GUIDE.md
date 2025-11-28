# Cloud SaaS MVP - Complete Implementation Guide

Comprehensive guide for deploying and operating MockForge as a fully managed cloud SaaS service with multi-tenant architecture, auto-scaling, and enterprise features.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Multi-Tenant System](#multi-tenant-system)
- [Auto-Scaling Configuration](#auto-scaling-configuration)
- [Tenant Provisioning](#tenant-provisioning)
- [Resource Management](#resource-management)
- [Billing Integration](#billing-integration)
- [Monitoring & Observability](#monitoring--observability)
- [Deployment Automation](#deployment-automation)
- [API Reference](#api-reference)
- [Operations](#operations)

---

## Overview

The MockForge Cloud SaaS MVP provides a fully managed, multi-tenant mock server platform with:

- **Multi-Tenant Architecture**: Complete tenant isolation with resource quotas
- **Auto-Scaling**: Automatic horizontal and vertical scaling based on demand
- **Billing Integration**: Stripe integration for subscriptions and usage-based billing
- **Resource Management**: Quota enforcement and usage tracking
- **High Availability**: 99.9% uptime SLA with multi-region support
- **Self-Service**: Tenant provisioning via API and UI

### Key Features

✅ **Multi-Tenancy**: Isolated workspaces per tenant
✅ **Auto-Scaling**: Kubernetes HPA with custom metrics
✅ **Billing**: Stripe subscriptions and usage tracking
✅ **Quotas**: Plan-based resource limits
✅ **API**: RESTful API for tenant management
✅ **UI**: Admin dashboard for tenant management
✅ **Monitoring**: Comprehensive observability stack

---

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Cloud Load Balancer                      │
│              (Multi-region, SSL termination)                │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
┌───────▼──────┐  ┌─────▼─────┐  ┌──────▼──────┐
│   Region 1   │  │  Region 2  │  │  Region 3   │
│  (Primary)   │  │ (Secondary)│  │ (Secondary) │
├──────────────┤  ├────────────┤  ├─────────────┤
│              │  │            │  │             │
│ ┌──────────┐ │  │ ┌────────┐ │  │ ┌─────────┐ │
│ │ API      │ │  │ │ API    │ │  │ │ API     │ │
│ │ Servers  │ │  │ │Servers │ │  │ │ Servers │ │
│ │ (Auto-   │ │  │ │(Auto-  │ │  │ │(Auto-   │ │
│ │ scaling) │ │  │ │scaling)│ │  │ │scaling) │ │
│ └────┬─────┘ │  │ └───┬────┘ │  │ └────┬────┘ │
│      │       │  │     │      │  │      │      │
│ ┌────▼─────┐ │  │ ┌───▼────┐ │  │ ┌────▼────┐ │
│ │ Admin UI │ │  │ │Admin UI│ │  │ │Admin UI │ │
│ └──────────┘ │  │ └────────┘ │  │ └─────────┘ │
│              │  │            │  │             │
└──────┬───────┘  └─────┬──────┘  └──────┬──────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
┌───────▼──────┐ ┌──────▼──────┐ ┌─────▼──────┐
│  PostgreSQL  │ │    Redis    │ │     S3     │
│  (Multi-AZ)  │ │  (Cluster)  │ │  (Storage) │
│              │ │             │ │            │
│ ┌──────────┐ │ │ ┌────────┐ │ │ ┌────────┐ │
│ │Tenant DB │ │ │ │ Cache  │ │ │ │Configs │ │
│ │(Schemas) │ │ │ │Sessions│ │ │ │Assets  │ │
│ └──────────┘ │ │ └────────┘ │ │ └────────┘ │
└──────────────┘ └─────────────┘ └────────────┘
```

### Component Overview

1. **API Layer**: Stateless MockForge instances (auto-scaling)
2. **Admin UI**: React-based management interface
3. **Database**: PostgreSQL with schema-per-tenant isolation
4. **Cache**: Redis cluster for sessions and rate limiting
5. **Storage**: S3-compatible storage for configurations and assets
6. **Load Balancer**: Multi-region load balancing with health checks

---

## Multi-Tenant System

### Tenant Isolation Strategy

MockForge uses **schema-per-tenant** isolation for complete data separation:

```sql
-- Each tenant gets its own schema
CREATE SCHEMA tenant_abc123;
CREATE SCHEMA tenant_def456;

-- Tenant-specific tables
CREATE TABLE tenant_abc123.workspaces (...);
CREATE TABLE tenant_abc123.mocks (...);
CREATE TABLE tenant_abc123.routes (...);
```

### Tenant Configuration

```yaml
# Tenant configuration
tenant:
  id: "abc123"
  name: "Acme Corporation"
  plan: "professional"
  status: "active"
  created_at: "2024-01-01T00:00:00Z"

  # Resource quotas
  quotas:
    max_workspaces: 100
    max_mocks: 1000
    max_requests_per_minute: 10000
    max_storage_mb: 5000
    max_users: 25

  # Permissions
  permissions:
    can_use_ai_features: true
    can_use_advanced_protocols: true
    can_export_data: true
    can_integrate_external: true
```

### Workspace Routing

Tenants access their workspaces via subdomain or path:

**Subdomain Routing:**
```
https://acme-corp.mockforge.dev/api/users
https://tenant-abc123.mockforge.dev/api/users
```

**Path-Based Routing:**
```
https://mockforge.dev/tenant/abc123/api/users
https://mockforge.dev/t/abc123/api/users  (short form)
```

---

## Auto-Scaling Configuration

### Kubernetes HPA (Horizontal Pod Autoscaler)

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: mockforge-api
  namespace: mockforge
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: mockforge-api
  minReplicas: 3
  maxReplicas: 50
  metrics:
  # CPU-based scaling
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  # Memory-based scaling
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  # Request rate scaling
  - type: Pods
    pods:
      metric:
        name: http_requests_per_second
      target:
        type: AverageValue
        averageValue: "1000"
  # Custom metric: active tenants
  - type: Pods
    pods:
      metric:
        name: active_tenants
      target:
        type: AverageValue
        averageValue: "10"
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
      - type: Pods
        value: 2
        periodSeconds: 60
      selectPolicy: Min
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

### Custom Metrics Exporter

```rust
// Expose custom metrics for HPA
use prometheus::{Counter, Gauge, Histogram, register_counter, register_gauge, register_histogram};

lazy_static! {
    static ref ACTIVE_TENANTS: Gauge = register_gauge!(
        "mockforge_active_tenants",
        "Number of active tenants"
    ).unwrap();

    static ref REQUESTS_PER_SECOND: Counter = register_counter!(
        "mockforge_requests_per_second",
        "Requests per second"
    ).unwrap();

    static ref TENANT_REQUEST_DURATION: Histogram = register_histogram!(
        "mockforge_tenant_request_duration_seconds",
        "Request duration by tenant"
    ).unwrap();
}

// Update metrics
ACTIVE_TENANTS.set(active_tenant_count as f64);
REQUESTS_PER_SECOND.inc();
TENANT_REQUEST_DURATION.observe(duration.as_secs_f64());
```

### Cluster Autoscaler

**AWS EKS:**
```yaml
apiVersion: autoscaling/v2
kind: ClusterAutoscaler
metadata:
  name: cluster-autoscaler
spec:
  scaleDownDelayAfterAdd: 10m
  scaleDownUnneededTime: 10m
  scaleDownUtilizationThreshold: 0.5
  skipNodesWithLocalStorage: false
  minReplicas: 3
  maxReplicas: 100
```

**GKE:**
```bash
gcloud container clusters update mockforge-cluster \
  --enable-autoscaling \
  --min-nodes 3 \
  --max-nodes 100 \
  --zone us-central1-a
```

---

## Tenant Provisioning

### Provisioning API

**Create Tenant:**

```bash
POST /api/v1/tenants
Content-Type: application/json
Authorization: Bearer <admin-token>

{
  "name": "Acme Corporation",
  "subdomain": "acme-corp",
  "plan": "professional",
  "admin_email": "admin@acme.com",
  "admin_name": "John Doe"
}
```

**Response:**
```json
{
  "tenant_id": "abc123",
  "name": "Acme Corporation",
  "subdomain": "acme-corp",
  "plan": "professional",
  "status": "active",
  "api_key": "sk_live_...",
  "created_at": "2024-01-01T00:00:00Z",
  "quotas": {
    "max_workspaces": 100,
    "max_mocks": 1000,
    "max_requests_per_minute": 10000
  }
}
```

**Get Tenant:**

```bash
GET /api/v1/tenants/{tenant_id}
Authorization: Bearer <admin-token>
```

**Update Tenant:**

```bash
PATCH /api/v1/tenants/{tenant_id}
Content-Type: application/json
Authorization: Bearer <admin-token>

{
  "plan": "enterprise",
  "status": "active"
}
```

**List Tenants:**

```bash
GET /api/v1/tenants?page=1&limit=50&status=active
Authorization: Bearer <admin-token>
```

### Provisioning Implementation

```rust
// Tenant provisioning service
pub struct TenantProvisioningService {
    db: Pool<Postgres>,
    redis: RedisPool,
    s3: S3Client,
}

impl TenantProvisioningService {
    pub async fn create_tenant(
        &self,
        request: CreateTenantRequest,
    ) -> Result<Tenant, ProvisioningError> {
        // 1. Validate request
        self.validate_tenant_request(&request).await?;

        // 2. Create database schema
        let schema_name = format!("tenant_{}", request.tenant_id);
        self.create_tenant_schema(&schema_name).await?;

        // 3. Create tenant record
        let tenant = self.create_tenant_record(&request).await?;

        // 4. Initialize default workspace
        self.create_default_workspace(&tenant.id).await?;

        // 5. Create S3 bucket/prefix
        self.create_tenant_storage(&tenant.id).await?;

        // 6. Generate API key
        let api_key = self.generate_api_key(&tenant.id).await?;

        // 7. Send welcome email
        self.send_welcome_email(&tenant, &api_key).await?;

        Ok(tenant)
    }

    async fn create_tenant_schema(&self, schema_name: &str) -> Result<()> {
        sqlx::query(&format!(
            "CREATE SCHEMA IF NOT EXISTS {}",
            schema_name
        ))
        .execute(&self.db)
        .await?;

        // Create tenant-specific tables
        sqlx::query(&format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.workspaces (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT NOW()
            )
            "#,
            schema_name
        ))
        .execute(&self.db)
        .await?;

        Ok(())
    }
}
```

---

## Resource Management

### Quota Enforcement

```rust
// Quota enforcement middleware
pub async fn quota_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract tenant from request
    let tenant_id = extract_tenant_id(&request)?;

    // Get tenant quotas
    let quotas = get_tenant_quotas(&tenant_id).await?;

    // Check request rate limit
    if !check_rate_limit(&tenant_id, &quotas).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Check workspace limit
    if !check_workspace_limit(&tenant_id, &quotas).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check storage limit
    if !check_storage_limit(&tenant_id, &quotas).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Process request
    let response = next.run(request).await;

    // Update usage metrics
    update_usage_metrics(&tenant_id, &response).await?;

    Ok(response)
}
```

### Usage Tracking

```rust
// Usage tracking service
pub struct UsageTrackingService {
    db: Pool<Postgres>,
    redis: RedisPool,
}

impl UsageTrackingService {
    pub async fn track_request(
        &self,
        tenant_id: &str,
        endpoint: &str,
        duration: Duration,
    ) -> Result<()> {
        // Increment request counter (Redis for speed)
        self.redis
            .incr(&format!("usage:{}:requests", tenant_id))
            .await?;

        // Track per-minute rate (for quota enforcement)
        self.redis
            .incr(&format!("usage:{}:rpm:{}", tenant_id, current_minute()))
            .await
            .ok(); // Don't fail on Redis errors

        // Store detailed metrics (async, non-blocking)
        tokio::spawn({
            let db = self.db.clone();
            let tenant_id = tenant_id.to_string();
            let endpoint = endpoint.to_string();
            let duration_ms = duration.as_millis() as i64;

            async move {
                sqlx::query(
                    r#"
                    INSERT INTO usage_metrics (tenant_id, endpoint, duration_ms, created_at)
                    VALUES ($1, $2, $3, NOW())
                    "#
                )
                .bind(&tenant_id)
                .bind(&endpoint)
                .bind(duration_ms)
                .execute(&db)
                .await
                .ok();
            }
        });

        Ok(())
    }

    pub async fn get_usage(
        &self,
        tenant_id: &str,
        period: UsagePeriod,
    ) -> Result<UsageStats> {
        let start_time = period.start_time();
        let end_time = period.end_time();

        let stats = sqlx::query_as!(
            UsageStats,
            r#"
            SELECT
                COUNT(*) as total_requests,
                AVG(duration_ms) as avg_duration_ms,
                MAX(duration_ms) as max_duration_ms,
                COUNT(DISTINCT endpoint) as unique_endpoints
            FROM usage_metrics
            WHERE tenant_id = $1
              AND created_at BETWEEN $2 AND $3
            "#,
            tenant_id,
            start_time,
            end_time
        )
        .fetch_one(&self.db)
        .await?;

        Ok(stats)
    }
}
```

---

## Billing Integration

### Stripe Integration

MockForge integrates with Stripe for subscription and usage-based billing:

```rust
// Billing service
pub struct BillingService {
    stripe: StripeClient,
    db: Pool<Postgres>,
}

impl BillingService {
    pub async fn create_subscription(
        &self,
        tenant_id: &str,
        plan: Plan,
    ) -> Result<Subscription> {
        // Create Stripe customer
        let customer = self.stripe
            .customers()
            .create(&CreateCustomer {
                email: Some(tenant.email.clone()),
                metadata: Some({
                    let mut map = HashMap::new();
                    map.insert("tenant_id".to_string(), tenant_id.to_string());
                    map
                }),
                ..Default::default()
            })
            .await?;

        // Create subscription
        let subscription = self.stripe
            .subscriptions()
            .create(&CreateSubscription {
                customer: customer.id.clone(),
                items: Some(vec![CreateSubscriptionItems {
                    price: plan.stripe_price_id.clone(),
                    ..Default::default()
                }]),
                ..Default::default()
            })
            .await?;

        // Store subscription in database
        self.store_subscription(tenant_id, &subscription).await?;

        Ok(subscription)
    }

    pub async fn record_usage(
        &self,
        tenant_id: &str,
        usage_items: Vec<UsageItem>,
    ) -> Result<()> {
        // Record usage in Stripe
        for item in usage_items {
            self.stripe
                .subscription_items()
                .create_usage_record(
                    &item.subscription_item_id,
                    &CreateUsageRecord {
                        quantity: item.quantity,
                        timestamp: Some(Utc::now().timestamp()),
                        ..Default::default()
                    },
                )
                .await?;
        }

        Ok(())
    }
}
```

### Usage-Based Billing

```yaml
# Billing configuration
billing:
  stripe:
    secret_key: ${STRIPE_SECRET_KEY}
    webhook_secret: ${STRIPE_WEBHOOK_SECRET}

  plans:
    free:
      price_id: null
      base_price: 0
      usage_based: false

    professional:
      price_id: price_abc123
      base_price: 49.00
      usage_based: true
      usage_items:
        - name: "API Requests"
          price_id: price_usage_requests
          unit: "1000 requests"
          price: 0.01
        - name: "Storage"
          price_id: price_usage_storage
          unit: "GB"
          price: 0.10
```

---

## Monitoring & Observability

### Key Metrics

**Tenant Metrics:**
- `mockforge_tenant_requests_total{tenant_id, status}`
- `mockforge_tenant_request_duration_seconds{tenant_id}`
- `mockforge_tenant_active_workspaces{tenant_id}`
- `mockforge_tenant_quota_usage{tenant_id, quota_type}`

**System Metrics:**
- `mockforge_active_tenants`
- `mockforge_total_requests_per_second`
- `mockforge_database_connections`
- `mockforge_cache_hit_rate`

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "MockForge Cloud SaaS",
    "panels": [
      {
        "title": "Active Tenants",
        "targets": [{
          "expr": "mockforge_active_tenants"
        }]
      },
      {
        "title": "Requests per Second",
        "targets": [{
          "expr": "rate(mockforge_tenant_requests_total[5m])"
        }]
      },
      {
        "title": "Top Tenants by Requests",
        "targets": [{
          "expr": "topk(10, sum by (tenant_id) (rate(mockforge_tenant_requests_total[5m])))"
        }]
      }
    ]
  }
}
```

---

## Deployment Automation

### Infrastructure as Code (Terraform)

```hcl
# terraform/main.tf
module "mockforge_saas" {
  source = "./modules/mockforge-saas"

  environment = "production"
  region      = "us-east-1"

  # Kubernetes cluster
  cluster_config = {
    min_nodes = 3
    max_nodes = 100
    node_type = "t3.large"
  }

  # Database
  database_config = {
    instance_class = "db.r5.large"
    multi_az       = true
    backup_retention = 30
  }

  # Redis
  redis_config = {
    node_type = "cache.r5.large"
    num_nodes = 3
  }

  # Auto-scaling
  autoscaling = {
    min_replicas = 3
    max_replicas = 50
    target_cpu   = 70
    target_memory = 80
  }
}
```

### CI/CD Pipeline

```yaml
# .github/workflows/deploy-saas.yml
name: Deploy Cloud SaaS

on:
  push:
    branches: [main]
    paths:
      - 'cloud-service/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build Docker image
        run: |
          docker build -t mockforge-saas:latest .
          docker tag mockforge-saas:latest ${{ secrets.REGISTRY }}/mockforge-saas:${{ github.sha }}
          docker push ${{ secrets.REGISTRY }}/mockforge-saas:${{ github.sha }}

      - name: Deploy to Kubernetes
        run: |
          kubectl set image deployment/mockforge-api \
            mockforge-api=${{ secrets.REGISTRY }}/mockforge-saas:${{ github.sha }} \
            -n mockforge

      - name: Run database migrations
        run: |
          kubectl run migration-job \
            --image=${{ secrets.REGISTRY }}/mockforge-saas:${{ github.sha }} \
            --command -- mockforge migrate \
            -n mockforge
```

---

## API Reference

### Tenant Management API

**Base URL:** `https://api.mockforge.dev/v1`

#### Create Tenant

```http
POST /tenants
Authorization: Bearer <admin-token>
Content-Type: application/json

{
  "name": "Acme Corporation",
  "subdomain": "acme-corp",
  "plan": "professional",
  "admin_email": "admin@acme.com"
}
```

#### Get Tenant

```http
GET /tenants/{tenant_id}
Authorization: Bearer <admin-token>
```

#### Update Tenant

```http
PATCH /tenants/{tenant_id}
Authorization: Bearer <admin-token>
Content-Type: application/json

{
  "plan": "enterprise",
  "status": "active"
}
```

#### Get Tenant Usage

```http
GET /tenants/{tenant_id}/usage?period=month
Authorization: Bearer <admin-token>
```

#### List Tenants

```http
GET /tenants?page=1&limit=50&status=active
Authorization: Bearer <admin-token>
```

### Tenant API (Self-Service)

**Base URL:** `https://api.mockforge.dev/v1` (tenant-scoped)

#### Get Workspaces

```http
GET /workspaces
Authorization: Bearer <tenant-api-key>
```

#### Create Workspace

```http
POST /workspaces
Authorization: Bearer <tenant-api-key>
Content-Type: application/json

{
  "name": "Production",
  "description": "Production environment"
}
```

#### Get Usage Stats

```http
GET /usage?period=month
Authorization: Bearer <tenant-api-key>
```

---

## Operations

### Daily Operations

1. **Monitor Tenant Activity**
   - Check active tenant count
   - Monitor request rates
   - Review quota usage

2. **Scale Resources**
   - Adjust HPA min/max replicas
   - Scale database if needed
   - Add Redis nodes if cache hit rate drops

3. **Review Billing**
   - Check subscription status
   - Review usage-based charges
   - Process failed payments

### Weekly Operations

1. **Capacity Planning**
   - Review growth trends
   - Plan for upcoming demand
   - Optimize resource allocation

2. **Security Review**
   - Audit tenant access
   - Review API key usage
   - Check for anomalies

3. **Performance Optimization**
   - Analyze slow queries
   - Optimize cache strategies
   - Review auto-scaling behavior

### Monthly Operations

1. **Cost Analysis**
   - Review cloud costs
   - Optimize resource usage
   - Plan cost reductions

2. **Compliance**
   - Review audit logs
   - Update security policies
   - Compliance reporting

3. **Feature Planning**
   - Review tenant feedback
   - Plan new features
   - Update quotas and plans

---

## Quick Start

### 1. Deploy Infrastructure

```bash
cd cloud-service/infrastructure/aws/production
terraform init
terraform apply
```

### 2. Deploy Application

```bash
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secrets.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/hpa.yaml
kubectl apply -f k8s/ingress.yaml
```

### 3. Create First Tenant

```bash
curl -X POST https://api.mockforge.dev/v1/tenants \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Tenant",
    "subdomain": "test",
    "plan": "professional",
    "admin_email": "admin@test.com"
  }'
```

### 4. Verify Deployment

```bash
# Check pods
kubectl get pods -n mockforge

# Check HPA
kubectl get hpa -n mockforge

# Check services
kubectl get svc -n mockforge

# View logs
kubectl logs -f deployment/mockforge-api -n mockforge
```

---

## Best Practices

1. **Start Small**: Begin with 3 replicas, scale as needed
2. **Monitor First**: Set up monitoring before scaling
3. **Test Failover**: Regularly test disaster recovery
4. **Automate Everything**: Use Infrastructure as Code
5. **Security First**: Enable all security features from start
6. **Cost Monitor**: Track costs and optimize regularly
7. **Document Everything**: Keep runbooks up to date
8. **Regular Backups**: Automated backups with tested restore

---

## Additional Resources

- [Enterprise Deployment Guide](ENTERPRISE_DEPLOYMENT_GUIDE.md)
- [Multi-Tenant Implementation](MULTI_TENANT_COMPLETE.md)
- [Billing Integration](../CLOUD_MONETIZATION_STATUS.md)
- [Kubernetes Deployment](CLOUD_DEPLOYMENT.md#kubernetes-deployment)
- [Monitoring Setup](ADVANCED_OBSERVABILITY.md)

---

**Last Updated**: 2024-01-01
**Version**: 1.0
