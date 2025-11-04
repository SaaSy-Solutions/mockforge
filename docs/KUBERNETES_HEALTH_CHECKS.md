# Kubernetes Health Checks & Deployment Readiness

This document describes MockForge's Kubernetes-native health check endpoints and deployment configuration.

## Health Check Endpoints

MockForge provides three health check endpoints following Kubernetes best practices:

### 1. Liveness Probe (`/health/live`)

**Purpose**: Indicates if the container is alive and should not be restarted.

- **Status 200**: Container is alive (not failed)
- **Status 503**: Container has failed and should be restarted

**When to use**: Kubernetes uses this to determine if a container should be restarted.

**Example Response**:
```json
{
  "status": "alive",
  "timestamp": "2025-01-27T12:00:00Z",
  "uptime_seconds": 3600,
  "version": "0.2.6"
}
```

### 2. Readiness Probe (`/health/ready`)

**Purpose**: Indicates if the container is ready to accept traffic.

- **Status 200**: Service is ready to accept requests
- **Status 503**: Service is not ready (initializing, shutting down, or failed)

**When to use**: Kubernetes uses this to determine if traffic should be routed to the pod.

**Example Response (Ready)**:
```json
{
  "status": "ready",
  "timestamp": "2025-01-27T12:00:00Z",
  "uptime_seconds": 3600,
  "version": "0.2.6",
  "details": {
    "initialization": "complete"
  }
}
```

**Example Response (Not Ready)**:
```json
{
  "status": "not_ready",
  "timestamp": "2025-01-27T12:00:00Z",
  "uptime_seconds": 5,
  "version": "0.2.6",
  "details": {
    "initialization": "initializing"
  }
}
```

### 3. Startup Probe (`/health/startup`)

**Purpose**: Indicates if the container has finished initialization.

- **Status 200**: Initialization is complete
- **Status 503**: Still initializing or failed

**When to use**: For services that take a long time to start. Kubernetes will not start the liveness/readiness probes until startup succeeds.

**Example Response**:
```json
{
  "status": "startup_complete",
  "timestamp": "2025-01-27T12:00:00Z",
  "uptime_seconds": 10,
  "version": "0.2.6",
  "details": {
    "initialization": "complete"
  }
}
```

### 4. Combined Health Check (`/health`)

**Purpose**: Backwards-compatible general health check endpoint.

Combines liveness and readiness checks. For Kubernetes deployments, prefer using the specific probe endpoints.

## Kubernetes Deployment Configuration

### Example Deployment YAML

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
        image: mockforge:0.2.6
        ports:
        - containerPort: 3000
          name: http
        # Startup probe: Wait up to 60 seconds for initialization
        startupProbe:
          httpGet:
            path: /health/startup
            port: 3000
          initialDelaySeconds: 0
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 12  # 60 seconds total
        # Liveness probe: Check every 30 seconds
        livenessProbe:
          httpGet:
            path: /health/live
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 30
          timeoutSeconds: 3
          failureThreshold: 3
        # Readiness probe: Check every 10 seconds
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 3
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### Service Configuration

```yaml
apiVersion: v1
kind: Service
metadata:
  name: mockforge-service
spec:
  selector:
    app: mockforge
  ports:
  - port: 80
    targetPort: 3000
    protocol: TCP
    name: http
  type: LoadBalancer
```

## Health Check Configuration

### Initialization Timeout

By default, services start in `Initializing` state. You can configure an initialization timeout:

```rust
use mockforge_http::HealthManager;
use std::time::Duration;

// Create health manager with 30-second initialization timeout
let health = Arc::new(
    HealthManager::with_init_timeout(Duration::from_secs(30))
);
```

### Marking Service as Ready

After all services are initialized, mark the service as ready:

```rust
// After HTTP server is listening
health.set_ready().await;

// After all services (HTTP, WebSocket, gRPC) are ready
health.set_ready().await;
```

### Graceful Shutdown

To trigger graceful shutdown:

```rust
// Mark service as shutting down (readiness probe will return 503)
health.trigger_shutdown().await;
```

## Service Status States

The health manager tracks the following states:

- **`Initializing`**: Service is starting up (not ready)
- **`Ready`**: Service is ready to accept traffic
- **`ShuttingDown`**: Service is shutting down (not accepting new requests)
- **`Failed`**: Service has failed (unhealthy)

## Integration with MockForge CLI

The health check endpoints are automatically available when using `mockforge serve`:

```bash
# Health endpoints are available at:
# - http://localhost:3000/health
# - http://localhost:3000/health/live
# - http://localhost:3000/health/ready
# - http://localhost:3000/health/startup

mockforge serve --http-port 3000
```

## Best Practices

1. **Startup Probe**: Use for services that take more than 30 seconds to start
2. **Liveness Probe**: Keep intervals reasonable (30-60 seconds) to avoid unnecessary restarts
3. **Readiness Probe**: Use shorter intervals (10 seconds) for faster traffic routing decisions
4. **Timeouts**: Set appropriate timeouts (3-5 seconds) to avoid hanging probes
5. **Failure Thresholds**: Configure based on expected failure scenarios (3 failures = restart)

## Troubleshooting

### Service Stuck in Initializing State

If the service remains in `Initializing` state:

1. Check if `health.set_ready()` is being called
2. Verify initialization timeout is not too short
3. Check logs for initialization errors

### Readiness Probe Failing

If readiness probe returns 503:

1. Verify all required services are initialized
2. Check if service is in `ShuttingDown` state
3. Review service initialization logs

### Liveness Probe Failing

If liveness probe returns 503:

1. Service has been marked as `Failed`
2. Check for critical errors in logs
3. Review resource constraints (memory, CPU)

## Additional Resources

- [Kubernetes Probes Documentation](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/)
- [MockForge Deployment Guide](docs/deployment/kubernetes.md)
- [Health Check API Reference](https://docs.rs/mockforge-http)
