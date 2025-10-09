# MockForge Deployment Templates

This directory contains deployment templates and configurations for various environments.

## Docker Compose Templates

### 1. Minimal Setup (`docker-compose.minimal.yml`)

Quick start for basic API mocking with HTTP and Admin UI only.

```bash
cd deploy
docker-compose -f docker-compose.minimal.yml up
```

**Use case:** Quick testing, development, demos

### 2. CI/CD Setup (`docker-compose.ci.yml`)

Optimized for CI/CD pipelines with fast startup and automated testing.

```bash
cd deploy
docker-compose -f docker-compose.ci.yml up --abort-on-container-exit
```

**Use case:** GitHub Actions, GitLab CI, Jenkins pipelines

### 3. Production Setup (`docker-compose.production.yml`)

Full production configuration with Nginx reverse proxy, SSL, resource limits, and monitoring.

```bash
cd deploy

# Configure SSL certificates in nginx/ssl/
# Edit nginx/nginx.conf for your domain

docker-compose -f docker-compose.production.yml up -d
```

**Use case:** Production deployments on VPS, bare metal, or cloud VMs

### 4. Full Observability (`docker-compose.yml`)

Complete observability stack with Prometheus, Grafana, and Jaeger.

```bash
cd deploy
docker-compose up -d
```

**Use case:** Testing with full observability, development with metrics

## Kubernetes/Helm Deployment

### Install via Helm

```bash
# Add MockForge Helm repository (when published)
helm repo add mockforge https://charts.mockforge.dev
helm repo update

# Install with default values
helm install mockforge mockforge/mockforge

# Install with custom values
helm install mockforge mockforge/mockforge -f custom-values.yaml

# Or install from local chart
helm install mockforge ../helm/mockforge
```

### Example Custom Values

```yaml
# custom-values.yaml
replicaCount: 5

image:
  repository: ghcr.io/saasy-solutions/mockforge
  tag: "1.0.0"

ingress:
  enabled: true
  className: nginx
  hosts:
    - host: api.example.com
      paths:
        - path: /
          pathType: Prefix
          backend: http

config:
  http:
    enabled: true
  admin:
    enabled: true
  observability:
    metrics:
      enabled: true
    tracing:
      enabled: true
      otlp_endpoint: "http://tempo:4317"

persistence:
  enabled: true
  size: 20Gi
```

### Helm Commands

```bash
# List releases
helm list

# Get release values
helm get values mockforge

# Upgrade release
helm upgrade mockforge mockforge/mockforge -f custom-values.yaml

# Rollback release
helm rollback mockforge 1

# Uninstall release
helm uninstall mockforge
```

## Cloud Platform Guides

### AWS ECS/Fargate

See [docs/deployment/aws.md](../../docs/deployment/aws.md)

### Google Cloud Run

See [docs/deployment/gcp.md](../../docs/deployment/gcp.md)

### Azure Container Instances

See [docs/deployment/azure.md](../../docs/deployment/azure.md)

### DigitalOcean App Platform

See [docs/deployment/digitalocean.md](../../docs/deployment/digitalocean.md)

## Nginx Configuration

The production setup includes Nginx as a reverse proxy with:

- **SSL/TLS termination** with modern cipher suites
- **Rate limiting** for API and admin endpoints
- **WebSocket support** for real-time connections
- **Security headers** (HSTS, X-Frame-Options, etc.)
- **IP whitelisting** for admin UI (optional)

### SSL Certificate Setup

```bash
# Option 1: Self-signed for testing
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout deploy/nginx/ssl/key.pem \
  -out deploy/nginx/ssl/cert.pem

# Option 2: Let's Encrypt with certbot
certbot certonly --standalone -d api.mockforge.example.com
cp /etc/letsencrypt/live/api.mockforge.example.com/fullchain.pem deploy/nginx/ssl/cert.pem
cp /etc/letsencrypt/live/api.mockforge.example.com/privkey.pem deploy/nginx/ssl/key.pem
```

## Environment Variables

Key environment variables for production:

```bash
# Core settings
MOCKFORGE_HTTP_PORT=3000
MOCKFORGE_WS_PORT=3001
MOCKFORGE_GRPC_PORT=50051
MOCKFORGE_ADMIN_PORT=9080

# Feature flags
MOCKFORGE_ADMIN_ENABLED=true
MOCKFORGE_METRICS_ENABLED=true
MOCKFORGE_TRACING_ENABLED=true
MOCKFORGE_RECORDER_ENABLED=true

# Observability
MOCKFORGE_OTLP_ENDPOINT=http://tempo:4317
MOCKFORGE_JAEGER_ENDPOINT=http://jaeger:14268/api/traces

# Storage
MOCKFORGE_RECORDER_DB=/app/data/mockforge.db
MOCKFORGE_FIXTURES_DIR=/app/fixtures

# Logging
RUST_LOG=info
```

## Resource Requirements

### Minimum

- CPU: 0.5 cores
- Memory: 256 MB
- Storage: 1 GB

### Recommended (Production)

- CPU: 1-2 cores
- Memory: 512 MB - 1 GB
- Storage: 10 GB (with recorder enabled)

### High Availability

- CPU: 2+ cores per instance
- Memory: 1 GB per instance
- Storage: 20+ GB (shared or replicated)
- Replicas: 3-5 instances

## Health Checks

MockForge provides health check endpoints:

- `/health/live` - Liveness probe (is the service running?)
- `/health/ready` - Readiness probe (is the service ready to serve traffic?)
- `/health/startup` - Startup probe (has the service finished initialization?)
- `/ping` - Simple ping endpoint

```bash
# Check liveness
curl http://localhost:9080/health/live

# Check readiness
curl http://localhost:9080/health/ready
```

## Monitoring

### Prometheus Metrics

MockForge exposes Prometheus metrics on port 9090:

```bash
curl http://localhost:9090/metrics
```

### Grafana Dashboards

When using the observability stack, Grafana is available at http://localhost:3002:

- Default username: admin
- Default password: admin

### Distributed Tracing

Jaeger UI is available at http://localhost:16686 for viewing distributed traces.

## Troubleshooting

### Check container logs

```bash
docker logs mockforge
docker-compose logs -f mockforge
kubectl logs -l app.kubernetes.io/name=mockforge
```

### Verify connectivity

```bash
# HTTP endpoint
curl http://localhost:3000/ping

# Admin UI
curl http://localhost:9080/health/live

# Metrics
curl http://localhost:9090/metrics
```

### Debug container

```bash
# Docker
docker exec -it mockforge /bin/sh

# Kubernetes
kubectl exec -it deployment/mockforge -- /bin/sh
```

## Security Considerations

1. **Admin UI Access:** Restrict admin UI to internal networks only
2. **SSL/TLS:** Always use HTTPS in production
3. **Rate Limiting:** Configure appropriate rate limits for your use case
4. **Network Policies:** Use network policies in Kubernetes to restrict traffic
5. **Secrets Management:** Use Docker secrets, Kubernetes secrets, or cloud secret managers
6. **Regular Updates:** Keep Docker images updated with latest security patches

## Support

For issues or questions:
- GitHub Issues: https://github.com/SaaSy-Solutions/mockforge/issues
- Documentation: https://mockforge.dev
