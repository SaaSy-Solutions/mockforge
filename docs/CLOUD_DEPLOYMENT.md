# MockForge Cloud-Native Deployment Guide

Complete guide for deploying MockForge in cloud-native environments including Kubernetes, Docker, and major cloud providers.

## Table of Contents

- [Quick Start](#quick-start)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Helm Chart](#helm-chart)
- [Cloud Providers](#cloud-providers)
- [Scaling](#scaling)
- [Monitoring](#monitoring)
- [Best Practices](#best-practices)

---

## Quick Start

### Using Docker Compose

The fastest way to get started:

```bash
cd deploy
docker-compose up -d
```

This starts:
- **MockForge** on ports 3000 (HTTP), 3001 (WS), 50051 (gRPC), 9080 (Admin)
- **Jaeger** on port 16686 (UI)
- **Prometheus** on port 9091
- **Grafana** on port 3002

Access:
- Admin UI: http://localhost:9080
- Jaeger UI: http://localhost:16686
- Grafana: http://localhost:3002 (admin/admin)

---

## Docker Deployment

### Building the Image

```bash
# Build from source
docker build -t mockforge:latest .

# Or pull from registry
docker pull mockforge/mockforge:latest
```

### Running a Container

```bash
docker run -d \
  --name mockforge \
  -p 3000:3000 \
  -p 3001:3001 \
  -p 50051:50051 \
  -p 9080:9080 \
  -e MOCKFORGE_ADMIN_ENABLED=true \
  -e MOCKFORGE_METRICS_ENABLED=true \
  -v $(pwd)/config.yaml:/app/config.yaml \
  mockforge:latest
```

### Configuration

Mount a config file:

```bash
docker run -d \
  -v $(pwd)/config.yaml:/app/config.yaml \
  mockforge:latest
```

Or use environment variables:

```bash
docker run -d \
  -e MOCKFORGE_HTTP_PORT=3000 \
  -e MOCKFORGE_ADMIN_ENABLED=true \
  -e MOCKFORGE_CHAOS_ENABLED=true \
  -e RUST_LOG=info \
  mockforge:latest
```

### Health Checks

Docker health check example:

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:9080/health/live"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 40s
```

---

## Kubernetes Deployment

### Prerequisites

- Kubernetes cluster (1.19+)
- kubectl configured
- Storage provisioner (for persistent volumes)

### Basic Deployment

#### 1. Apply Kubernetes Manifests

```bash
# Create namespace
kubectl create namespace mockforge

# Apply all manifests
kubectl apply -f k8s/ -n mockforge
```

This creates:
- Deployment with 3 replicas
- Services (ClusterIP, headless)
- ConfigMap for configuration
- PersistentVolumeClaim for data
- ServiceAccount and RBAC
- HorizontalPodAutoscaler

#### 2. Verify Deployment

```bash
# Check pods
kubectl get pods -n mockforge

# Check services
kubectl get services -n mockforge

# Check logs
kubectl logs -f deployment/mockforge -n mockforge
```

#### 3. Access the Service

Port-forward for local access:

```bash
# Admin UI
kubectl port-forward -n mockforge svc/mockforge 9080:9080

# HTTP API
kubectl port-forward -n mockforge svc/mockforge 3000:3000
```

### Production Deployment

#### 1. Configure Ingress

Update `k8s/ingress.yaml` with your domain:

```yaml
spec:
  tls:
  - hosts:
    - mockforge.your-domain.com
    secretName: mockforge-tls
  rules:
  - host: mockforge.your-domain.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: mockforge
            port:
              name: http
```

Create TLS certificate:

```bash
# Using cert-manager
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: mockforge-tls
  namespace: mockforge
spec:
  secretName: mockforge-tls
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
  - mockforge.your-domain.com
EOF
```

Apply ingress:

```bash
kubectl apply -f k8s/ingress.yaml -n mockforge
```

#### 2. Configure Monitoring

Install Prometheus Operator:

```bash
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install prometheus prometheus-community/kube-prometheus-stack -n monitoring --create-namespace
```

Apply ServiceMonitor:

```bash
kubectl apply -f k8s/servicemonitor.yaml -n mockforge
```

---

## Helm Chart

### Installation

#### 1. Add Helm Repository (when available)

```bash
helm repo add mockforge https://charts.mockforge.dev
helm repo update
```

#### 2. Install Chart

```bash
# Basic installation
helm install mockforge mockforge/mockforge -n mockforge --create-namespace

# With custom values
helm install mockforge mockforge/mockforge -n mockforge \
  --set replicaCount=5 \
  --set ingress.enabled=true \
  --set ingress.hosts[0].host=mockforge.example.com
```

#### 3. Using Local Chart

```bash
# Install from local directory
helm install mockforge ./helm/mockforge -n mockforge --create-namespace
```

### Configuration

Create a `values.yaml` file:

```yaml
replicaCount: 5

image:
  repository: mockforge/mockforge
  tag: "1.0.0"

ingress:
  enabled: true
  className: nginx
  hosts:
    - host: mockforge.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: mockforge-tls
      hosts:
        - mockforge.example.com

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 20
  targetCPUUtilizationPercentage: 70

config:
  observability:
    metrics:
      enabled: true
    tracing:
      enabled: true
      otlp_endpoint: "http://jaeger-collector:4317"
    chaos:
      enabled: true

persistence:
  enabled: true
  size: 50Gi
```

Install with custom values:

```bash
helm install mockforge ./helm/mockforge -n mockforge -f values.yaml
```

### Upgrading

```bash
# Upgrade to new version
helm upgrade mockforge ./helm/mockforge -n mockforge

# Rollback if needed
helm rollback mockforge -n mockforge
```

### Uninstalling

```bash
helm uninstall mockforge -n mockforge
```

---

## Cloud Providers

### AWS (EKS)

#### 1. Create EKS Cluster

```bash
eksctl create cluster \
  --name mockforge-cluster \
  --region us-west-2 \
  --node-type t3.medium \
  --nodes 3 \
  --nodes-min 3 \
  --nodes-max 10 \
  --managed
```

#### 2. Configure kubectl

```bash
aws eks update-kubeconfig --region us-west-2 --name mockforge-cluster
```

#### 3. Install AWS Load Balancer Controller

```bash
helm repo add eks https://aws.github.io/eks-charts
helm install aws-load-balancer-controller eks/aws-load-balancer-controller \
  -n kube-system \
  --set clusterName=mockforge-cluster
```

#### 4. Deploy MockForge

```bash
helm install mockforge ./helm/mockforge -n mockforge --create-namespace
```

#### 5. Configure Load Balancer

Update ingress annotations in `values.yaml`:

```yaml
ingress:
  enabled: true
  className: alb
  annotations:
    alb.ingress.kubernetes.io/scheme: internet-facing
    alb.ingress.kubernetes.io/target-type: ip
    alb.ingress.kubernetes.io/healthcheck-path: /health/ready
```

#### 6. Use EBS for Persistence

```yaml
persistence:
  enabled: true
  storageClass: gp3
  size: 50Gi
```

### GCP (GKE)

#### 1. Create GKE Cluster

```bash
gcloud container clusters create mockforge-cluster \
  --zone us-central1-a \
  --machine-type n1-standard-2 \
  --num-nodes 3 \
  --enable-autoscaling \
  --min-nodes 3 \
  --max-nodes 10
```

#### 2. Configure kubectl

```bash
gcloud container clusters get-credentials mockforge-cluster --zone us-central1-a
```

#### 3. Deploy MockForge

```bash
helm install mockforge ./helm/mockforge -n mockforge --create-namespace
```

#### 4. Configure Load Balancer

For external access:

```yaml
service:
  type: LoadBalancer
  annotations:
    cloud.google.com/load-balancer-type: "External"
```

#### 5. Use GCE Persistent Disk

```yaml
persistence:
  enabled: true
  storageClass: standard-rwo
  size: 50Gi
```

### Azure (ASK)

#### 1. Create ASK Cluster

```bash
az ask create \
  --resource-group mockforge-rg \
  --name mockforge-cluster \
  --node-count 3 \
  --enable-addons monitoring \
  --generate-ssh-keys
```

#### 2. Configure kubectl

```bash
az ask get-credentials --resource-group mockforge-rg --name mockforge-cluster
```

#### 3. Deploy MockForge

```bash
helm install mockforge ./helm/mockforge -n mockforge --create-namespace
```

#### 4. Configure Application Gateway

```yaml
ingress:
  enabled: true
  className: azure-application-gateway
  annotations:
    appgw.ingress.kubernetes.io/ssl-redirect: "true"
```

#### 5. Use Azure Disk

```yaml
persistence:
  enabled: true
  storageClass: managed-premium
  size: 50Gi
```

---

## Scaling

### Horizontal Scaling

#### Manual Scaling

```bash
# Scale deployment
kubectl scale deployment mockforge -n mockforge --replicas=10
```

#### Auto-Scaling (HPA)

The HPA is configured in `k8s/hpa.yaml`:

```yaml
spec:
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

Monitor HPA:

```bash
kubectl get hpa -n mockforge
kubectl describe hpa mockforge -n mockforge
```

#### Custom Metrics

For request-based scaling:

```yaml
metrics:
- type: Pods
  pods:
    metric:
      name: http_requests_per_second
    target:
      type: AverageValue
      averageValue: "1000"
```

### Vertical Scaling

Update resource limits:

```yaml
resources:
  limits:
    cpu: 1000m
    memory: 1Gi
  requests:
    cpu: 500m
    memory: 512Mi
```

### Cluster Auto-Scaling

#### AWS

```bash
eksctl create cluster \
  --name mockforge-cluster \
  --nodes-min 3 \
  --nodes-max 50 \
  --node-type t3.large \
  --enable-asg
```

#### GKE

```bash
gcloud container clusters update mockforge-cluster \
  --enable-autoscaling \
  --min-nodes 3 \
  --max-nodes 50 \
  --zone us-central1-a
```

#### ASK

```bash
az ask update \
  --resource-group mockforge-rg \
  --name mockforge-cluster \
  --enable-cluster-autoscaler \
  --min-count 3 \
  --max-count 50
```

---

## Monitoring

### Prometheus

Metrics are exposed at `/metrics` on port 9090.

#### Scrape Configuration

```yaml
scrape_configs:
  - job_name: 'mockforge'
    kubernetes_sd_configs:
    - role: pod
      namespaces:
        names:
        - mockforge
    relabel_configs:
    - source_labels: [__meta_kubernetes_pod_label_app]
      action: keep
      regex: mockforge
    - source_labels: [__meta_kubernetes_pod_container_port_name]
      action: keep
      regex: metrics
```

### Grafana Dashboards

Import pre-built dashboards:

1. Navigate to Grafana
2. Import dashboard from `deploy/grafana/dashboards/`
3. Select Prometheus data source

### Distributed Tracing

#### Jaeger

Configure OTLP endpoint:

```yaml
config:
  observability:
    tracing:
      enabled: true
      otlp_endpoint: "http://jaeger-collector:4317"
```

Deploy Jaeger:

```bash
kubectl apply -f - <<EOF
apiVersion: apps/v1
kind: Deployment
metadata:
  name: jaeger
  namespace: mockforge
spec:
  replicas: 1
  selector:
    matchLabels:
      app: jaeger
  template:
    metadata:
      labels:
        app: jaeger
    spec:
      containers:
      - name: jaeger
        image: jaegertracing/all-in-one:latest
        ports:
        - containerPort: 16686
          name: ui
        - containerPort: 4317
          name: otlp-grpc
EOF
```

### Logging

#### Fluent Bit

Deploy Fluent Bit for log aggregation:

```bash
helm repo add fluent https://fluent.github.io/helm-charts
helm install fluent-bit fluent/fluent-bit -n logging --create-namespace
```

---

## Best Practices

### 1. Resource Management

**Set appropriate resource limits**:

```yaml
resources:
  limits:
    cpu: 500m
    memory: 512Mi
  requests:
    cpu: 250m
    memory: 256Mi
```

### 2. High Availability

**Run multiple replicas**:

```yaml
replicaCount: 3  # Minimum for HA
```

**Use pod anti-affinity**:

```yaml
affinity:
  podAntiAffinity:
    preferredDuringSchedulingIgnoredDuringExecution:
    - weight: 100
      podAffinityTerm:
        labelSelector:
          matchLabels:
            app: mockforge
        topologyKey: kubernetes.io/hostname
```

### 3. Security

**Run as non-root**:

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  fsGroup: 1000
```

**Use network policies**:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: mockforge
spec:
  podSelector:
    matchLabels:
      app: mockforge
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - podSelector: {}
    ports:
    - protocol: TCP
      port: 3000
```

### 4. Data Persistence

**Use persistent volumes**:

```yaml
persistence:
  enabled: true
  storageClass: standard
  size: 50Gi
```

### 5. Health Checks

**Configure all three probes**:

```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: admin
readinessProbe:
  httpGet:
    path: /health/ready
    port: admin
startupProbe:
  httpGet:
    path: /health/startup
    port: admin
```

### 6. Configuration Management

**Use ConfigMaps and Secrets**:

```bash
kubectl create configmap mockforge-config --from-file=config.yaml
kubectl create secret generic mockforge-secrets --from-literal=api-key=xxx
```

### 7. Rolling Updates

**Configure deployment strategy**:

```yaml
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxUnavailable: 1
    maxSurge: 1
```

### 8. Backup and Disaster Recovery

**Backup persistent volumes**:

```bash
# Using Velero
velero backup create mockforge-backup --include-namespaces mockforge
```

---

## Troubleshooting

### Pods Not Starting

```bash
# Check events
kubectl get events -n mockforge --sort-by='.lastTimestamp'

# Check pod logs
kubectl logs -n mockforge deployment/mockforge

# Describe pod
kubectl describe pod -n mockforge <pod-name>
```

### Service Not Accessible

```bash
# Check service
kubectl get svc -n mockforge

# Test from within cluster
kubectl run -it --rm debug --image=curlimages/curl --restart=Never -n mockforge -- sh
curl http://mockforge:3000/health/live
```

### High Memory Usage

```bash
# Check resource usage
kubectl top pods -n mockforge

# Increase limits if needed
kubectl set resources deployment mockforge -n mockforge --limits=memory=1Gi
```

---

## Additional Resources

- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Helm Documentation](https://helm.sh/docs/)
- [AWS EKS Documentation](https://docs.aws.amazon.com/eks/)
- [GCP GKE Documentation](https://cloud.google.com/kubernetes-engine/docs)
- [Azure ASK Documentation](https://docs.microsoft.com/en-us/azure/aks/)
- [MockForge Documentation](https://docs.mockforge.dev)

---

## Support

For issues or questions:
- GitHub Issues: https://github.com/SaaSy-Solutions/mockforge/issues
- GitHub Discussions: https://github.com/SaaSy-Solutions/mockforge/discussions
- Discord: https://discord.gg/2FxXqKpa
- Documentation: https://docs.mockforge.dev
