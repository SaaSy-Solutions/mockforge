# MockForge Helm Chart

Official Helm chart for deploying MockForge on Kubernetes.

## Introduction

MockForge is an advanced API mocking and chaos engineering platform. This Helm chart deploys MockForge to a Kubernetes cluster with production-ready defaults.

## Prerequisites

- Kubernetes 1.19+
- Helm 3.2.0+
- PV provisioner support in the underlying infrastructure (optional, for persistent storage)

## Installing the Chart

### From GitHub Pages (when published)

```bash
# Add the repository
helm repo add mockforge https://saasy-solutions.github.io/mockforge/charts
helm repo update

# Install with default values
helm install mockforge mockforge/mockforge

# Install with custom values
helm install mockforge mockforge/mockforge -f custom-values.yaml
```

### From Local Directory

```bash
# Install from local chart
helm install mockforge ./helm/mockforge

# Install with custom values
helm install mockforge ./helm/mockforge -f custom-values.yaml
```

### Quick Start Examples

**Minimal Installation:**
```bash
helm install mockforge mockforge/mockforge \
  --set replicaCount=1
```

**Production Installation:**
```bash
helm install mockforge mockforge/mockforge \
  --set replicaCount=3 \
  --set ingress.enabled=true \
  --set ingress.hosts[0].host=api.example.com \
  --set persistence.enabled=true \
  --set persistence.size=20Gi
```

**With Monitoring:**
```bash
helm install mockforge mockforge/mockforge \
  --set serviceMonitor.enabled=true \
  --set config.observability.metrics.enabled=true \
  --set config.observability.tracing.enabled=true \
  --set config.observability.tracing.otlp_endpoint=http://tempo:4317
```

## Uninstalling the Chart

```bash
helm uninstall mockforge
```

## Configuration

The following table lists the configurable parameters of the MockForge chart and their default values.

### Global Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of MockForge replicas | `3` |
| `image.repository` | MockForge image repository | `mockforge/mockforge` |
| `image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `image.tag` | Image tag (defaults to chart appVersion) | `latest` |
| `imagePullSecrets` | Docker registry secret names | `[]` |
| `nameOverride` | Override chart name | `""` |
| `fullnameOverride` | Override full release name | `""` |

### Service Account

| Parameter | Description | Default |
|-----------|-------------|---------|
| `serviceAccount.create` | Create service account | `true` |
| `serviceAccount.annotations` | Service account annotations | `{}` |
| `serviceAccount.name` | Service account name | `""` |

### Pod Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `podAnnotations` | Pod annotations | `{"prometheus.io/scrape": "true", ...}` |
| `podSecurityContext.runAsNonRoot` | Run as non-root user | `true` |
| `podSecurityContext.runAsUser` | User ID | `1000` |
| `podSecurityContext.fsGroup` | File system group | `1000` |
| `securityContext.readOnlyRootFilesystem` | Read-only root filesystem | `false` |
| `securityContext.allowPrivilegeEscalation` | Allow privilege escalation | `false` |

### Service Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `service.type` | Service type | `ClusterIP` |
| `service.http.port` | HTTP port | `3000` |
| `service.websocket.port` | WebSocket port | `3001` |
| `service.grpc.port` | gRPC port | `50051` |
| `service.admin.port` | Admin UI port | `9080` |
| `service.metrics.port` | Metrics port | `9090` |

### Ingress Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `ingress.enabled` | Enable ingress | `false` |
| `ingress.className` | Ingress class name | `nginx` |
| `ingress.annotations` | Ingress annotations | `{}` |
| `ingress.hosts` | Ingress hosts configuration | See values.yaml |
| `ingress.tls` | Ingress TLS configuration | `[]` |

### Resource Limits

| Parameter | Description | Default |
|-----------|-------------|---------|
| `resources.limits.cpu` | CPU limit | `500m` |
| `resources.limits.memory` | Memory limit | `512Mi` |
| `resources.requests.cpu` | CPU request | `250m` |
| `resources.requests.memory` | Memory request | `256Mi` |

### Autoscaling

| Parameter | Description | Default |
|-----------|-------------|---------|
| `autoscaling.enabled` | Enable HPA | `true` |
| `autoscaling.minReplicas` | Minimum replicas | `3` |
| `autoscaling.maxReplicas` | Maximum replicas | `20` |
| `autoscaling.targetCPUUtilizationPercentage` | Target CPU % | `70` |
| `autoscaling.targetMemoryUtilizationPercentage` | Target Memory % | `80` |

### MockForge Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `config.http.enabled` | Enable HTTP server | `true` |
| `config.http.port` | HTTP port | `3000` |
| `config.websocket.enabled` | Enable WebSocket server | `true` |
| `config.websocket.port` | WebSocket port | `3001` |
| `config.grpc.enabled` | Enable gRPC server | `true` |
| `config.grpc.port` | gRPC port | `50051` |
| `config.admin.enabled` | Enable Admin UI | `true` |
| `config.admin.port` | Admin UI port | `9080` |
| `config.observability.metrics.enabled` | Enable metrics | `true` |
| `config.observability.tracing.enabled` | Enable tracing | `true` |
| `config.observability.recorder.enabled` | Enable recorder | `true` |
| `config.chaos.enabled` | Enable chaos engineering | `true` |
| `config.latency.base_ms` | Base latency in ms | `50` |
| `config.failures.enabled` | Enable failure injection | `false` |

### Persistence

| Parameter | Description | Default |
|-----------|-------------|---------|
| `persistence.enabled` | Enable persistent storage | `true` |
| `persistence.storageClass` | Storage class | `""` |
| `persistence.accessMode` | Access mode | `ReadWriteOnce` |
| `persistence.size` | Storage size | `10Gi` |

### Monitoring

| Parameter | Description | Default |
|-----------|-------------|---------|
| `serviceMonitor.enabled` | Enable Prometheus ServiceMonitor | `false` |
| `serviceMonitor.interval` | Scrape interval | `30s` |
| `serviceMonitor.scrapeTimeout` | Scrape timeout | `10s` |

## Examples

### Custom Values File

Create `my-values.yaml`:

```yaml
replicaCount: 5

image:
  repository: ghcr.io/saasy-solutions/mockforge
  tag: "1.0.0"

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
  hosts:
    - host: api.example.com
      paths:
        - path: /
          pathType: Prefix
          backend: http
  tls:
    - secretName: mockforge-tls
      hosts:
        - api.example.com

config:
  observability:
    metrics:
      enabled: true
    tracing:
      enabled: true
      otlp_endpoint: "http://tempo.observability.svc:4317"
    recorder:
      enabled: true
      database_path: "/app/data/mockforge.db"

persistence:
  enabled: true
  storageClass: "fast-ssd"
  size: 20Gi

serviceMonitor:
  enabled: true

resources:
  limits:
    cpu: 1000m
    memory: 1Gi
  requests:
    cpu: 500m
    memory: 512Mi
```

Install with:
```bash
helm install mockforge mockforge/mockforge -f my-values.yaml
```

### Using with Different Ingress Controllers

**Nginx Ingress:**
```bash
helm install mockforge mockforge/mockforge \
  --set ingress.enabled=true \
  --set ingress.className=nginx
```

**Traefik:**
```bash
helm install mockforge mockforge/mockforge \
  --set ingress.enabled=true \
  --set ingress.className=traefik
```

**AWS ALB:**
```bash
helm install mockforge mockforge/mockforge \
  --set ingress.enabled=true \
  --set ingress.className=alb \
  --set ingress.annotations."alb\.ingress\.kubernetes\.io/scheme"=internet-facing
```

**GCE:**
```bash
helm install mockforge mockforge/mockforge \
  --set ingress.enabled=true \
  --set ingress.className=gce
```

## Upgrading

```bash
# Update repository
helm repo update

# Upgrade release
helm upgrade mockforge mockforge/mockforge

# Upgrade with new values
helm upgrade mockforge mockforge/mockforge -f new-values.yaml
```

## Troubleshooting

### View Pod Logs

```bash
kubectl logs -l app.kubernetes.io/name=mockforge -f
```

### Check Pod Status

```bash
kubectl get pods -l app.kubernetes.io/name=mockforge
kubectl describe pod <pod-name>
```

### Check Service

```bash
kubectl get svc mockforge
kubectl describe svc mockforge
```

### Check Ingress

```bash
kubectl get ingress
kubectl describe ingress mockforge
```

### Port Forward for Local Testing

```bash
# Forward HTTP port
kubectl port-forward svc/mockforge 3000:3000

# Forward Admin UI port
kubectl port-forward svc/mockforge 9080:9080
```

## Security Considerations

1. **Non-root User**: The container runs as a non-root user (UID 1000)
2. **Read-only Root Filesystem**: Consider enabling for enhanced security
3. **Network Policies**: Implement network policies to restrict traffic
4. **RBAC**: Use appropriate service account with minimal permissions
5. **Secrets**: Use Kubernetes secrets for sensitive configuration
6. **Ingress TLS**: Always use TLS in production

## Performance Tuning

### High Traffic

```yaml
replicaCount: 10

autoscaling:
  enabled: true
  minReplicas: 10
  maxReplicas: 50
  targetCPUUtilizationPercentage: 60

resources:
  limits:
    cpu: 2000m
    memory: 2Gi
  requests:
    cpu: 1000m
    memory: 1Gi
```

### Low Latency

```yaml
config:
  latency:
    base_ms: 10
    jitter_percent: 5

affinity:
  podAntiAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
    - labelSelector:
        matchExpressions:
        - key: app.kubernetes.io/name
          operator: In
          values:
          - mockforge
      topologyKey: kubernetes.io/hostname
```

## Support

- GitHub Issues: https://github.com/SaaSy-Solutions/mockforge/issues
- Documentation: https://mockforge.dev
- Helm Chart Repo: https://saasy-solutions.github.io/mockforge/charts
