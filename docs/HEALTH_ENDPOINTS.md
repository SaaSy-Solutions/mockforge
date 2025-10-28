# Health & Readiness Endpoints

MockForge provides comprehensive health check endpoints for monitoring, orchestration platforms (Kubernetes, Docker Swarm), and CI/CD pipelines.

## Available Endpoints

MockForge supports **two naming conventions** for health endpoints:

### 1. RESTful Style (Recommended)

| Endpoint | Purpose | Use Case |
|----------|---------|----------|
| `/health` | Deep health check | Comprehensive system health |
| `/health/live` | Liveness probe | Is the application alive? |
| `/health/ready` | Readiness probe | Ready to serve traffic? |
| `/health/startup` | Startup probe | Initialization complete? |

### 2. Kubernetes Style (Aliases)

| Endpoint | Alias For | Purpose |
|----------|-----------|---------|
| `/healthz` | `/health` | Deep health check |
| `/livez` | `/health/live` | Liveness probe |
| `/readyz` | `/health/ready` | Readiness probe |
| `/startupz` | `/health/startup` | Startup probe |

Both conventions are **functionally identical** - use whichever fits your infrastructure conventions.

## Endpoint Details

### `/health` or `/healthz` - Deep Health Check

**Purpose:** Comprehensive system health check for monitoring and diagnostics.

**Returns:** Detailed health status of all subsystems

**Example Request:**
```bash
curl http://localhost:8081/healthz
```

**Example Response:**
```json
{
  "status": "healthy",
  "timestamp": 1761104965,
  "version": "0.1.2",
  "uptime_seconds": 120,
  "checks": [
    {
      "name": "http_server",
      "status": "healthy",
      "message": "http_server is running",
      "duration_ms": 0
    },
    {
      "name": "websocket_server",
      "status": "healthy",
      "message": "websocket_server is running",
      "duration_ms": 0
    },
    {
      "name": "grpc_server",
      "status": "healthy",
      "message": "grpc_server is running",
      "duration_ms": 0
    },
    {
      "name": "metrics",
      "status": "healthy",
      "message": "Processed 1234 requests",
      "duration_ms": 0
    }
  ]
}
```

**Use Cases:**
- Monitoring dashboards (Grafana, Datadog, etc.)
- Diagnostic scripts
- Health check aggregators

---

### `/health/ready` or `/readyz` - Readiness Probe

**Purpose:** Determine if the application is ready to accept traffic.

**Returns:** Status based on critical service availability

**Example Request:**
```bash
curl http://localhost:8081/readyz
```

**Example Response:**
```json
{
  "status": "healthy",
  "timestamp": 1761104966,
  "version": "0.1.2",
  "uptime_seconds": 10,
  "checks": [
    {
      "name": "http_server",
      "status": "healthy",
      "message": "HTTP server is running",
      "duration_ms": 0
    },
    {
      "name": "websocket_server",
      "status": "healthy",
      "message": "WebSocket server is running",
      "duration_ms": 0
    },
    {
      "name": "grpc_server",
      "status": "healthy",
      "message": "gRPC server is running",
      "duration_ms": 0
    }
  ]
}
```

**Status Values:**
- `"healthy"` - All critical services are ready
- `"degraded"` - Some non-critical services unavailable
- `"unhealthy"` - Critical services failed

**Use Cases:**
- Kubernetes readiness probes
- Load balancer health checks
- Service mesh readiness gates

**Kubernetes Example:**
```yaml
readinessProbe:
  httpGet:
    path: /readyz
    port: 8081
  initialDelaySeconds: 5
  periodSeconds: 10
```

---

### `/health/live` or `/livez` - Liveness Probe

**Purpose:** Determine if the application is alive (not deadlocked or crashed).

**Returns:** Always `200 OK` if the application is running

**Example Request:**
```bash
curl http://localhost:8081/livez
```

**Example Response:**
```json
{
  "status": "healthy",
  "timestamp": 1761104967,
  "version": "0.1.2",
  "uptime_seconds": 15,
  "checks": []
}
```

**Use Cases:**
- Kubernetes liveness probes (restart on failure)
- Process supervisors (systemd, supervisord)
- Container orchestration platforms

**Kubernetes Example:**
```yaml
livenessProbe:
  httpGet:
    path: /livez
    port: 8081
  initialDelaySeconds: 30
  periodSeconds: 10
```

---

### `/health/startup` or `/startupz` - Startup Probe

**Purpose:** Determine if the application has completed initialization.

**Returns:** Status based on initialization completion

**Example Request:**
```bash
curl http://localhost:8081/startupz
```

**Example Response:**
```json
{
  "status": "healthy",
  "timestamp": 1761104968,
  "version": "0.1.2",
  "uptime_seconds": 5,
  "checks": [
    {
      "name": "initialization",
      "status": "healthy",
      "message": "Application initialized",
      "duration_ms": 0
    }
  ]
}
```

**Use Cases:**
- Kubernetes startup probes (for slow-starting containers)
- Deployment verification
- Initialization monitoring

**Kubernetes Example:**
```yaml
startupProbe:
  httpGet:
    path: /startupz
    port: 8081
  initialDelaySeconds: 0
  periodSeconds: 5
  failureThreshold: 30
```

---

## Integration Examples

### Docker Compose

```yaml
services:
  mockforge:
    image: mockforge:latest
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/healthz"]
      interval: 10s
      timeout: 3s
      retries: 3
      start_period: 10s
```

### Kubernetes Complete Example

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mockforge
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mockforge
  template:
    metadata:
      labels:
        app: mockforge
    spec:
      containers:
      - name: mockforge
        image: mockforge:latest
        ports:
        - containerPort: 3030
          name: http
        - containerPort: 8081
          name: admin

        # Liveness: Restart if application is deadlocked
        livenessProbe:
          httpGet:
            path: /livez
            port: 8081
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 3

        # Readiness: Remove from service if not ready
        readinessProbe:
          httpGet:
            path: /readyz
            port: 8081
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 3

        # Startup: Allow slow startup before liveness kicks in
        startupProbe:
          httpGet:
            path: /startupz
            port: 8081
          initialDelaySeconds: 0
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 30  # 150 seconds max startup time
```

### HAProxy

```
backend mockforge_backend
    option httpchk GET /readyz
    http-check expect status 200
    server mockforge1 192.168.1.10:3030 check port 8081
    server mockforge2 192.168.1.11:3030 check port 8081
```

### NGINX

```nginx
upstream mockforge {
    server 192.168.1.10:3030;
    server 192.168.1.11:3030;
}

server {
    listen 80;

    location / {
        proxy_pass http://mockforge;
    }

    # Health check endpoint
    location /health {
        access_log off;
        proxy_pass http://mockforge;
        proxy_set_header Host $host;
    }
}
```

### Prometheus Monitoring

```yaml
scrape_configs:
  - job_name: 'mockforge-health'
    metrics_path: '/healthz'
    scrape_interval: 30s
    static_configs:
      - targets: ['localhost:8081']
```

### curl Script

```bash
#!/bin/bash
# Health check script for monitoring

ADMIN_URL="http://localhost:8081"

# Check if service is ready
response=$(curl -s -o /dev/null -w "%{http_code}" "${ADMIN_URL}/readyz")

if [ "$response" -eq 200 ]; then
    echo "✅ MockForge is ready"
    exit 0
else
    echo "❌ MockForge is not ready (HTTP $response)"
    exit 1
fi
```

---

## Response Status Codes

| HTTP Code | Meaning | When Used |
|-----------|---------|-----------|
| `200 OK` | Healthy | Service is operational |
| `503 Service Unavailable` | Unhealthy | Service is down or not ready |

**Note:** Currently, all endpoints return `200 OK` with a JSON status field. Future versions may use HTTP status codes to indicate health.

---

## Dashboard Endpoints

### `/__mockforge/dashboard` or `/_mf` - Admin Dashboard

**Purpose:** Web-based dashboard with metrics, logs, and controls.

**Example Request:**
```bash
curl http://localhost:8081/_mf
```

**Example Response:**
```json
{
  "success": true,
  "data": {
    "server_info": {
      "version": "0.1.2",
      "build_time": "2025-10-22T03:48:36.091016444Z",
      "git_sha": "b1e7184",
      "http_server": "127.0.0.1:3030",
      "ws_server": "127.0.0.1:3001",
      "grpc_server": "127.0.0.1:50051",
      "api_enabled": true,
      "admin_port": 8081
    },
    "metrics": {
      "total_requests": 1234,
      "active_requests": 5,
      "average_response_time": 45.2,
      "error_rate": 0.02
    },
    "servers": [...],
    "recent_logs": [...],
    "system": {
      "uptime_seconds": 3600,
      "memory_usage_mb": 512,
      "cpu_usage_percent": 15.5,
      "active_threads": 32
    }
  }
}
```

**Features:**
- Active routes with request counts and latency
- Server status (HTTP, WebSocket, gRPC, GraphQL)
- Real-time metrics
- Recent request logs
- System information
- Scenario controls
- Replay controls (via time-travel API)

**Use Cases:**
- Local development monitoring
- Debug dashboards
- CI/CD verification
- Manual testing

**Browser Access:**
Open `http://localhost:8081/_mf` in your browser for the full admin UI.

---

## Best Practices

### 1. Use Different Endpoints for Different Purposes

```yaml
# Kubernetes - separate probes
livenessProbe:
  httpGet:
    path: /livez    # Restart if down

readinessProbe:
  httpGet:
    path: /readyz   # Remove from load balancer if not ready

startupProbe:
  httpGet:
    path: /startupz # Wait for startup before checking liveness
```

### 2. Set Appropriate Timeouts

```yaml
# Allow time for slow startup
startupProbe:
  failureThreshold: 30
  periodSeconds: 5
  # Total: 150 seconds max startup time

# Quick liveness checks
livenessProbe:
  periodSeconds: 10
  timeoutSeconds: 3
```

### 3. Monitor Health in CI/CD

```bash
# Wait for service to be ready before running tests
timeout 60 bash -c 'until curl -sf http://localhost:8081/readyz; do sleep 1; done'
echo "MockForge is ready for testing"
```

### 4. Use Short Aliases for Simplicity

```bash
# Kubernetes convention
curl http://localhost:8081/healthz

# RESTful convention
curl http://localhost:8081/health
```

Both are identical - choose based on your team's conventions.

---

## Troubleshooting

### Health Check Returns 503

**Cause:** Service is not ready

**Solutions:**
1. Check if servers started: `curl http://localhost:8081/livez`
2. Review startup logs
3. Increase startup timeout in probes

### Health Check Times Out

**Cause:** Admin UI not started or wrong port

**Solutions:**
1. Verify admin UI is enabled: `mockforge serve --admin`
2. Check correct port: `--admin-port 8081`
3. Verify firewall rules

### Probe Causing Restart Loop

**Cause:** Liveness probe too aggressive

**Solutions:**
1. Increase `initialDelaySeconds`
2. Increase `failureThreshold`
3. Use startup probe first

---

## Related Documentation

- [Dashboard & UI Documentation](./DASHBOARD.md)
- [Structured Logging](./STRUCTURED_LOGGING.md)
- [Observability](./OBSERVABILITY.md)
- [Kubernetes Deployment](../deploy/kubernetes/README.md)

---

## Summary

MockForge provides **8 health check endpoints** across two naming conventions:

| RESTful | Kubernetes | Purpose |
|---------|------------|---------|
| `/health` | `/healthz` | Deep health check |
| `/health/live` | `/livez` | Liveness probe |
| `/health/ready` | `/readyz` | Readiness probe |
| `/health/startup` | `/startupz` | Startup probe |

Plus:
- `/__mockforge/dashboard` - Full admin dashboard
- `/_mf` - Short dashboard alias

All endpoints return JSON with detailed health information and are ready for production use in any orchestration platform.
