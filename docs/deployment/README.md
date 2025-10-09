# MockForge Cloud Deployment Guides

Comprehensive guides for deploying MockForge on various cloud platforms and container services.

## Quick Links

- **[AWS Deployment](aws.md)** - Amazon Web Services (ECS, EKS, App Runner, EC2)
- **[GCP Deployment](gcp.md)** - Google Cloud Platform (Cloud Run, GKE, Compute Engine)
- **[Azure Deployment](azure.md)** - Microsoft Azure (Container Apps, ACI, AKS, VMs)
- **[DigitalOcean Deployment](digitalocean.md)** - DigitalOcean (App Platform, DOKS, Droplets)

## Platform Comparison

### Managed Container Services (Easiest)

| Platform | Service | Pricing | Best For |
|----------|---------|---------|----------|
| **Google Cloud** | Cloud Run | $10-30/mo | Serverless, auto-scaling, pay-per-use |
| **DigitalOcean** | App Platform | $10-20/mo | Simple deployments, fixed pricing |
| **Azure** | Container Apps | $15-35/mo | Azure ecosystem integration |
| **AWS** | App Runner | $45-60/mo | AWS ecosystem integration |

### Kubernetes Services (Production)

| Platform | Service | Pricing | Best For |
|----------|---------|---------|----------|
| **DigitalOcean** | DOKS | $84/mo | Affordable managed K8s |
| **AWS** | EKS | $183/mo | Enterprise AWS deployments |
| **Azure** | AKS | $228/mo | Enterprise Azure deployments |
| **Google Cloud** | GKE | $168-240/mo | Advanced GKE features |

### Virtual Machines (Traditional)

| Platform | Service | Pricing | Best For |
|----------|---------|---------|----------|
| **DigitalOcean** | Droplets | $24-98/mo | Simple VMs, predictable pricing |
| **AWS** | EC2 | $35/mo+ | Full AWS integration |
| **Azure** | VMs | $118/mo+ | Azure ecosystem |
| **Google Cloud** | Compute Engine | $103/mo+ | GCP integration |

## Recommendation by Use Case

### 1. **Getting Started / Development**
→ **DigitalOcean App Platform** ($10/month)
- Easiest setup
- Great documentation
- Predictable pricing
- [Deploy Guide](digitalocean.md#digitalocean-app-platform-recommended)

### 2. **Low-Cost Production**
→ **Google Cloud Run** ($10-30/month)
- Serverless (pay for what you use)
- Auto-scaling
- Easy HTTPS setup
- [Deploy Guide](gcp.md#google-cloud-run-recommended)

### 3. **High-Traffic Production**
→ **AWS ECS Fargate** or **Kubernetes** ($120-230/month)
- Auto-scaling
- High availability
- Enterprise features
- [AWS Guide](aws.md#aws-ecs-with-fargate) | [K8s Guide](#kubernetes-deployment)

### 4. **Enterprise / Complex Requirements**
→ **Kubernetes (AKS/EKS/GKE/DOKS)** ($84-240/month)
- Full orchestration
- Advanced networking
- Multi-region
- [Platform-specific K8s guides](#kubernetes-deployment)

### 5. **Maximum Control**
→ **Virtual Machines** ($24-120/month)
- Complete control
- Traditional infrastructure
- Custom configurations
- [VM Deployment Guides](#virtual-machine-deployment)

## Kubernetes Deployment

MockForge provides a production-ready Helm chart for Kubernetes deployment.

### Quick Start

```bash
# Add Helm repository (when published)
helm repo add mockforge https://saasy-solutions.github.io/mockforge/charts
helm repo update

# Install with default values
helm install mockforge mockforge/mockforge

# Or from local chart
helm install mockforge ./helm/mockforge
```

### Platform-Specific Guides

- [AWS EKS](aws.md#aws-eks-kubernetes)
- [Google GKE](gcp.md#google-kubernetes-engine-gke)
- [Azure AKS](azure.md#azure-kubernetes-service-aks)
- [DigitalOcean DOKS](digitalocean.md#digitalocean-kubernetes-doks)

### Helm Chart Documentation

See [Helm Chart README](../../helm/mockforge/README.md) for:
- Full configuration options
- Custom values examples
- Ingress setup
- Monitoring configuration
- Security best practices

## Docker Deployment

For Docker and Docker Compose deployments, see:
- [Docker Setup Guide](../../DOCKER.md)
- [Docker Compose Templates](../../deploy/)

### Available Templates

1. **Minimal** - Quick start (`deploy/docker-compose.minimal.yml`)
2. **CI/CD** - For pipelines (`deploy/docker-compose.ci.yml`)
3. **Production** - With Nginx, SSL (`deploy/docker-compose.production.yml`)
4. **Observability** - Full monitoring stack (`deploy/docker-compose.yml`)

## General Deployment Best Practices

### 1. **Configuration Management**

Always use environment variables or configuration files:

```bash
# Core settings
MOCKFORGE_HTTP_PORT=3000
MOCKFORGE_ADMIN_ENABLED=true

# Observability
MOCKFORGE_METRICS_ENABLED=true
MOCKFORGE_TRACING_ENABLED=true

# Storage
MOCKFORGE_RECORDER_DB=/app/data/mockforge.db
```

### 2. **Secrets Management**

Use platform-specific secret managers:
- AWS: AWS Secrets Manager or Parameter Store
- GCP: Secret Manager
- Azure: Key Vault
- Kubernetes: Kubernetes Secrets
- Docker: Docker Secrets

### 3. **Monitoring**

Enable observability features:
```yaml
config:
  observability:
    metrics:
      enabled: true
    tracing:
      enabled: true
      otlp_endpoint: "http://tempo:4317"
    recorder:
      enabled: true
```

### 4. **High Availability**

- Run at least 3 replicas
- Use pod anti-affinity rules
- Configure health checks
- Set up autoscaling

```yaml
replicaCount: 3
autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 20
```

### 5. **Security**

- Always use HTTPS in production
- Restrict admin UI access
- Use network policies
- Enable pod security policies
- Scan images for vulnerabilities

### 6. **Persistence**

For the recorder feature:
```yaml
persistence:
  enabled: true
  size: 20Gi
  storageClass: "fast-ssd"
```

### 7. **Resource Limits**

Set appropriate limits:
```yaml
resources:
  limits:
    cpu: 1000m
    memory: 1Gi
  requests:
    cpu: 500m
    memory: 512Mi
```

## Infrastructure as Code

### Terraform Examples

Each cloud guide includes Terraform examples:
- [AWS CloudFormation/Terraform](aws.md#using-cloudformation)
- [GCP Terraform](gcp.md#infrastructure-as-code-terraform)
- [Azure ARM Templates](azure.md#using-arm-template)

### Pulumi

MockForge can be deployed using Pulumi. Example:

```typescript
import * as k8s from "@pulumi/kubernetes";

const mockforge = new k8s.helm.v3.Chart("mockforge", {
    chart: "mockforge",
    repo: "https://saasy-solutions.github.io/mockforge/charts",
    values: {
        replicaCount: 3,
        ingress: { enabled: true }
    }
});
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Deploy to Kubernetes
  run: |
    helm upgrade --install mockforge mockforge/mockforge \
      --set image.tag=${{ github.sha }} \
      --wait
```

### GitLab CI

```yaml
deploy:
  image: alpine/helm
  script:
    - helm upgrade --install mockforge mockforge/mockforge
```

### CircleCI

```yaml
- run:
    name: Deploy MockForge
    command: |
      helm upgrade --install mockforge mockforge/mockforge
```

## Migration Between Platforms

### Backup Data

```bash
# Backup recorder database
kubectl cp <pod-name>:/app/data/mockforge.db ./backup/mockforge.db

# Backup fixtures
kubectl cp <pod-name>:/app/fixtures ./backup/fixtures
```

### Restore Data

```bash
# Restore to new deployment
kubectl cp ./backup/mockforge.db <new-pod-name>:/app/data/mockforge.db
kubectl cp ./backup/fixtures <new-pod-name>:/app/fixtures
```

## Support and Resources

- **Documentation**: https://mockforge.dev
- **GitHub**: https://github.com/SaaSy-Solutions/mockforge
- **Issues**: https://github.com/SaaSy-Solutions/mockforge/issues
- **Discussions**: https://github.com/SaaSy-Solutions/mockforge/discussions

## Next Steps

1. Choose your deployment platform
2. Follow the platform-specific guide
3. Configure monitoring and alerting
4. Set up CI/CD pipeline
5. Implement backup strategy
6. Review security checklist

## Contributing

Found an issue with a deployment guide? Have a better approach?

Please open an issue or submit a pull request at:
https://github.com/SaaSy-Solutions/mockforge
