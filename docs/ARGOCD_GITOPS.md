# ArgoCD GitOps Deployment Guide

## Overview

MockForge uses ArgoCD for GitOps-based continuous deployment to Kubernetes clusters.

## Architecture

```
┌─────────────┐      ┌──────────┐      ┌────────────┐
│   GitHub    │─────▶│  ArgoCD  │─────▶│ Kubernetes │
│ Repository  │      │  Server  │      │  Cluster   │
└─────────────┘      └──────────┘      └────────────┘
      │                    │
      │                    │
      ▼                    ▼
┌─────────────┐      ┌──────────┐
│   Docker    │      │  Image   │
│   Registry  │◀─────│ Updater  │
└─────────────┘      └──────────┘
```

## Installation

### Prerequisites

- Kubernetes cluster (1.24+)
- kubectl configured
- Helm 3 installed

### Install ArgoCD

```bash
# Create ArgoCD namespace
kubectl create namespace argocd

# Install ArgoCD
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml

# Wait for ArgoCD to be ready
kubectl wait --for=condition=available --timeout=600s \
  deployment/argocd-server -n argocd

# Get initial admin password
ARGO_PASSWORD=$(kubectl -n argocd get secret argocd-initial-admin-secret \
  -o jsonpath="{.data.password}" | base64 -d)

echo "ArgoCD admin password: $ARGO_PASSWORD"

# Port forward to access UI
kubectl port-forward svc/argocd-server -n argocd 8080:443
```

Access ArgoCD UI at: https://localhost:8080

### Install ArgoCD CLI

```bash
# Linux
curl -sSL -o /usr/local/bin/argocd \
  https://github.com/argoproj/argo-cd/releases/latest/download/argocd-linux-amd64
chmod +x /usr/local/bin/argocd

# macOS
brew install argocd

# Login
argocd login localhost:8080 --username admin --password $ARGO_PASSWORD
```

### Install ArgoCD Image Updater

```bash
kubectl apply -n argocd \
  -f https://raw.githubusercontent.com/argoproj-labs/argocd-image-updater/stable/manifests/install.yaml
```

### Install ArgoCD Notifications

```bash
kubectl apply -n argocd \
  -f https://raw.githubusercontent.com/argoproj-labs/argocd-notifications/stable/manifests/install.yaml
```

## Configuration

### 1. Configure Repository Access

```bash
# Add GitHub repository
argocd repo add https://github.com/YOUR_ORG/mockforge.git \
  --username YOUR_USERNAME \
  --password YOUR_TOKEN

# Or use SSH
argocd repo add git@github.com:YOUR_ORG/mockforge.git \
  --ssh-private-key-path ~/.ssh/id_rsa
```

### 2. Configure Docker Registry

```bash
# Add GitHub Container Registry credentials
kubectl create secret docker-registry ghcr-secret \
  --docker-server=ghcr.io \
  --docker-username=YOUR_USERNAME \
  --docker-password=YOUR_TOKEN \
  --namespace=mockforge

# Configure Image Updater
kubectl patch configmap argocd-image-updater-config \
  -n argocd \
  --type merge \
  -p '{"data":{"registries.conf":"registries:\n- name: GitHub Container Registry\n  prefix: ghcr.io\n  api_url: https://ghcr.io\n  credentials: secret:argocd/ghcr-secret"}}'
```

### 3. Create ArgoCD Application

```bash
# Deploy MockForge application
kubectl apply -f deploy/argocd/application.yaml

# Verify application
argocd app get mockforge
```

### 4. Configure Notifications

```bash
# Create Slack token secret
kubectl create secret generic argocd-notifications-secret \
  -n argocd \
  --from-literal=slack-token=YOUR_SLACK_TOKEN

# Apply notification configuration
kubectl apply -f deploy/argocd/application.yaml
```

## GitOps Workflow

### Deployment Process

1. **Developer pushes code** to GitHub
2. **GitHub Actions** builds Docker image
3. **Image is pushed** to GitHub Container Registry
4. **Image Updater** detects new image
5. **Git commit** updates manifest with new tag
6. **ArgoCD detects** manifest change
7. **ArgoCD syncs** to Kubernetes cluster

### Manual Sync

```bash
# Sync application
argocd app sync mockforge

# Hard refresh (ignore cache)
argocd app sync mockforge --force

# Preview sync (dry-run)
argocd app sync mockforge --dry-run
```

### Rollback

```bash
# View history
argocd app history mockforge

# Rollback to previous version
argocd app rollback mockforge

# Rollback to specific revision
argocd app rollback mockforge 5
```

## Multi-Environment Strategy

### Environment Structure

```
deploy/
├── argocd/
│   ├── application.yaml       # Production
│   ├── staging.yaml           # Staging
│   └── dev.yaml              # Development
helm/
└── mockforge/
    ├── values.yaml            # Base values
    ├── values-prod.yaml       # Production overrides
    ├── values-staging.yaml    # Staging overrides
    └── values-dev.yaml        # Dev overrides
```

### Create Environment Applications

```yaml
# Staging
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: mockforge-staging
  namespace: argocd
spec:
  source:
    repoURL: https://github.com/YOUR_ORG/mockforge.git
    targetRevision: develop
    path: helm/mockforge
    helm:
      valueFiles:
        - values.yaml
        - values-staging.yaml
  destination:
    server: https://kubernetes.default.svc
    namespace: mockforge-staging
```

## Progressive Delivery with Argo Rollouts

### Install Argo Rollouts

```bash
kubectl create namespace argo-rollouts
kubectl apply -n argo-rollouts \
  -f https://github.com/argoproj/argo-rollouts/releases/latest/download/install.yaml
```

### Canary Deployment

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Rollout
metadata:
  name: mockforge
spec:
  replicas: 10
  strategy:
    canary:
      steps:
      - setWeight: 10
      - pause: {duration: 5m}
      - setWeight: 25
      - pause: {duration: 5m}
      - setWeight: 50
      - pause: {duration: 5m}
      - setWeight: 75
      - pause: {duration: 5m}
  template:
    # ... pod template
```

### Blue-Green Deployment

```yaml
apiVersion: argoproj.io/v1alpha1
kind: Rollout
metadata:
  name: mockforge
spec:
  strategy:
    blueGreen:
      activeService: mockforge
      previewService: mockforge-preview
      autoPromotionEnabled: false
      prePromotionAnalysis:
        templates:
        - templateName: smoke-tests
```

## Monitoring and Observability

### ArgoCD Metrics

```bash
# View sync status
argocd app list

# View detailed status
argocd app get mockforge

# View application events
kubectl get events -n mockforge --sort-by='.lastTimestamp'
```

### Prometheus Queries

```promql
# Application sync status
argocd_app_info{project="mockforge"}

# Sync failures
rate(argocd_app_sync_total{phase="Failed"}[5m])

# Out of sync applications
argocd_app_sync_status{sync_status="OutOfSync"}
```

### Grafana Dashboard

Import ArgoCD dashboard: Dashboard ID `14584`

## Security Best Practices

1. **RBAC Configuration**
```yaml
# Limit user permissions
apiVersion: v1
kind: ConfigMap
metadata:
  name: argocd-rbac-cm
  namespace: argocd
data:
  policy.csv: |
    p, role:developer, applications, get, */*, allow
    p, role:developer, applications, sync, */*, allow
    g, developers, role:developer
```

2. **Enable SSO**
```yaml
# Example: GitHub OAuth
data:
  url: https://argocd.example.com
  dex.config: |
    connectors:
    - type: github
      id: github
      name: GitHub
      config:
        clientID: $GITHUB_CLIENT_ID
        clientSecret: $GITHUB_CLIENT_SECRET
        orgs:
        - name: YOUR_ORG
```

3. **Enable Audit Logging**
```bash
kubectl patch configmap argocd-cm -n argocd --type merge \
  -p '{"data":{"audit.log.enabled":"true"}}'
```

## Troubleshooting

### Application Out of Sync

```bash
# Check differences
argocd app diff mockforge

# View sync status
argocd app get mockforge --show-operation

# Force sync
argocd app sync mockforge --force
```

### Image Not Updating

```bash
# Check image updater logs
kubectl logs -n argocd -l app.kubernetes.io/name=argocd-image-updater

# Manually trigger update
argocd app set mockforge -p image.tag=v1.2.3
```

### Sync Failures

```bash
# View application logs
argocd app logs mockforge

# Check resource status
kubectl get all -n mockforge

# View ArgoCD server logs
kubectl logs -n argocd deployment/argocd-server
```

## Advanced Features

### App of Apps Pattern

```yaml
# Parent application
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: mockforge-apps
spec:
  source:
    path: deploy/argocd/apps
    # Contains multiple application manifests
```

### Sync Waves

```yaml
metadata:
  annotations:
    argocd.argoproj.io/sync-wave: "1"  # Deploy after wave 0
```

### Resource Hooks

```yaml
metadata:
  annotations:
    argocd.argoproj.io/hook: PreSync
    argocd.argoproj.io/hook-delete-policy: BeforeHookCreation
```

## Cost Optimization

- Use Image Updater to reduce manual operations
- Configure auto-sync for dev environments
- Use manual sync for production
- Implement progressive delivery to reduce rollback costs

## References

- [ArgoCD Documentation](https://argo-cd.readthedocs.io/)
- [ArgoCD Image Updater](https://argocd-image-updater.readthedocs.io/)
- [Argo Rollouts](https://argoproj.github.io/argo-rollouts/)
- [GitOps Principles](https://opengitops.dev/)
