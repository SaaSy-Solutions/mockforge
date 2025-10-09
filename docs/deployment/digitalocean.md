# Deploying MockForge on DigitalOcean

This guide covers deploying MockForge on DigitalOcean using various services.

## Table of Contents

- [DigitalOcean App Platform](#digitalocean-app-platform-recommended)
- [DigitalOcean Kubernetes (DOKS)](#digitalocean-kubernetes-doks)
- [DigitalOcean Droplets](#digitalocean-droplets)
- [Cost Estimation](#cost-estimation)

## DigitalOcean App Platform (Recommended)

The easiest and most cost-effective way to deploy MockForge on DigitalOcean.

### Prerequisites

- DigitalOcean account
- `doctl` CLI installed (optional)

### Option 1: Web Console Deployment

1. Go to [DigitalOcean App Platform](https://cloud.digitalocean.com/apps)
2. Click "Create App"
3. Choose "Docker Hub" as source
4. Enter image: `ghcr.io/saasy-solutions/mockforge:latest`
5. Configure:
   - **Container Port:** 3000
   - **HTTP Port:** 3000
   - **Instance Size:** Basic (512 MB RAM, 1 vCPU) - $5/month
   - **Instance Count:** 1-3
6. Add environment variables:
   ```
   MOCKFORGE_HTTP_PORT=3000
   MOCKFORGE_ADMIN_ENABLED=true
   MOCKFORGE_ADMIN_PORT=9080
   ```
7. Click "Create Resources"

### Option 2: CLI Deployment

Create `app.yaml`:

```yaml
name: mockforge
region: nyc
services:
- name: mockforge-service
  image:
    registry_type: GHCR
    registry: ghcr.io
    repository: saasy-solutions/mockforge
    tag: latest
  http_port: 3000
  instance_count: 2
  instance_size_slug: basic-xxs  # $5/month per instance
  routes:
  - path: /
  health_check:
    http_path: /ping
    initial_delay_seconds: 30
    period_seconds: 10
    timeout_seconds: 5
    success_threshold: 1
    failure_threshold: 3
  envs:
  - key: MOCKFORGE_HTTP_PORT
    value: "3000"
  - key: MOCKFORGE_ADMIN_ENABLED
    value: "true"
  - key: MOCKFORGE_ADMIN_PORT
    value: "9080"
  - key: RUST_LOG
    value: "info"
```

Deploy with:

```bash
doctl apps create --spec app.yaml
```

### Update Deployment

```bash
# Update app
doctl apps update YOUR_APP_ID --spec app.yaml

# View logs
doctl apps logs YOUR_APP_ID --type run

# Get app info
doctl apps get YOUR_APP_ID
```

## DigitalOcean Kubernetes (DOKS)

For production workloads requiring Kubernetes.

### Step 1: Create DOKS Cluster

Using `doctl`:

```bash
doctl kubernetes cluster create mockforge-cluster \
  --region nyc1 \
  --size s-2vcpu-4gb \
  --count 3 \
  --auto-upgrade=true \
  --maintenance-window "saturday=02:00"
```

Or via web console at [DigitalOcean Kubernetes](https://cloud.digitalocean.com/kubernetes/clusters).

### Step 2: Get Kubeconfig

```bash
doctl kubernetes cluster kubeconfig save mockforge-cluster
```

### Step 3: Deploy with Helm

```bash
# Install from local chart
helm install mockforge ./helm/mockforge \
  --set image.repository=ghcr.io/saasy-solutions/mockforge \
  --set image.tag=latest \
  --set ingress.enabled=true \
  --set ingress.className=nginx

# Or from repository (when published)
helm repo add mockforge https://charts.mockforge.dev
helm install mockforge mockforge/mockforge
```

### Step 4: Install Nginx Ingress Controller

```bash
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx
helm repo update

helm install nginx-ingress ingress-nginx/ingress-nginx \
  --set controller.service.type=LoadBalancer \
  --set controller.metrics.enabled=true
```

### Step 5: Get Load Balancer IP

```bash
kubectl get svc nginx-ingress-ingress-nginx-controller
```

### Step 6: Configure DNS

Point your domain to the LoadBalancer IP in DigitalOcean DNS or your DNS provider.

## DigitalOcean Droplets

Traditional VM deployment.

### Step 1: Create Droplet

Using `doctl`:

```bash
doctl compute droplet create mockforge-1 \
  --region nyc1 \
  --size s-2vcpu-4gb \
  --image ubuntu-22-04-x64 \
  --ssh-keys YOUR_SSH_KEY_ID \
  --user-data-file user-data.sh
```

### Step 2: User Data Script

Create `user-data.sh`:

```bash
#!/bin/bash

# Update system
apt-get update
apt-get upgrade -y

# Install Docker
curl -fsSL https://get.docker.com | sh
systemctl start docker
systemctl enable docker

# Pull and run MockForge
docker pull ghcr.io/saasy-solutions/mockforge:latest

docker run -d \
  --name mockforge \
  --restart unless-stopped \
  -p 80:3000 \
  -p 443:3000 \
  -p 9080:9080 \
  -e MOCKFORGE_HTTP_PORT=3000 \
  -e MOCKFORGE_ADMIN_ENABLED=true \
  -e MOCKFORGE_ADMIN_PORT=9080 \
  -e RUST_LOG=info \
  -v /opt/mockforge/data:/app/data \
  -v /opt/mockforge/fixtures:/app/fixtures \
  ghcr.io/saasy-solutions/mockforge:latest

# Configure firewall
ufw allow 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
ufw allow 9080/tcp
ufw --force enable

# Install monitoring agent
curl -sSL https://repos.insights.digitalocean.com/install.sh | bash
```

### Step 3: Create Multiple Droplets with Load Balancer

```bash
# Create 3 droplets
for i in {1..3}; do
  doctl compute droplet create mockforge-$i \
    --region nyc1 \
    --size s-2vcpu-4gb \
    --image ubuntu-22-04-x64 \
    --ssh-keys YOUR_SSH_KEY_ID \
    --user-data-file user-data.sh \
    --tag-names mockforge
done

# Create load balancer
doctl compute load-balancer create \
  --name mockforge-lb \
  --region nyc1 \
  --forwarding-rules "entry_protocol:http,entry_port:80,target_protocol:http,target_port:3000" \
  --health-check "protocol:http,port:3000,path:/ping,check_interval_seconds:10,response_timeout_seconds:5,healthy_threshold:3,unhealthy_threshold:3" \
  --tag-name mockforge
```

## Using DigitalOcean Container Registry

For private images:

```bash
# Create registry
doctl registry create mockforge-registry

# Log in to registry
doctl registry login

# Tag and push
docker tag ghcr.io/saasy-solutions/mockforge:latest \
  registry.digitalocean.com/mockforge-registry/mockforge:latest

docker push registry.digitalocean.com/mockforge-registry/mockforge:latest

# Configure Kubernetes to use registry
doctl registry kubernetes-manifest | kubectl apply -f -
```

## Using DigitalOcean Managed Databases

Add PostgreSQL for persistent storage:

```bash
# Create PostgreSQL database
doctl databases create mockforge-db \
  --engine pg \
  --region nyc1 \
  --size db-s-1vcpu-1gb \
  --num-nodes 1

# Get connection details
doctl databases connection mockforge-db --format Host,Port,User,Password
```

Update your deployment to use the database for the recorder feature.

## Docker Compose on Droplet

For a complete setup with docker-compose:

```bash
# SSH to droplet
ssh root@YOUR_DROPLET_IP

# Install docker-compose
curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Clone deployment config
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge/deploy

# Start with production compose
docker-compose -f docker-compose.production.yml up -d
```

## Cost Estimation

### App Platform (Basic XXS, 2 instances)
- **Instances:** $5/month × 2 = $10/month
- **Best for:** Development, low-traffic production
- **Cheapest and easiest option**

### DOKS (3 nodes, s-2vcpu-4gb)
- **Cluster:** Free
- **Worker nodes:** $24/month × 3 = $72/month
- **Load balancer:** $12/month
- **Total:** ~$84/month
- **Best for:** Production with orchestration needs

### Droplets (3 × s-2vcpu-4gb + Load Balancer)
- **Droplets:** $24/month × 3 = $72/month
- **Load balancer:** $12/month
- **Backups (optional):** $4.80/month × 3 = $14.40/month
- **Total:** ~$84-98/month
- **Best for:** Traditional VM-based deployments

### Single Droplet (s-2vcpu-4gb)
- **Droplet:** $24/month
- **Backup (optional):** $4.80/month
- **Total:** ~$24-29/month
- **Best for:** Small production, staging, development

## Best Practices

1. **Use App Platform** for simplest deployment
2. **Enable automatic backups** for Droplets
3. **Use Spaces** for object storage (fixtures, recordings)
4. **Enable monitoring** with DigitalOcean Monitoring
5. **Use Floating IPs** for high availability
6. **Implement regular snapshots** for disaster recovery
7. **Use VPC** for private networking between resources

## Monitoring

### DigitalOcean Monitoring

Enable built-in monitoring:

```bash
# Install monitoring agent on Droplet
curl -sSL https://repos.insights.digitalocean.com/install.sh | bash
```

View metrics in the DigitalOcean control panel.

### Set Up Alerts

```bash
# Create CPU alert
doctl monitoring alert create \
  --type v1/insights/droplet/cpu \
  --description "High CPU Alert" \
  --compare GreaterThan \
  --value 80 \
  --window 5m \
  --entities DROPLET_ID \
  --enabled true
```

## Troubleshooting

### View App Platform logs

```bash
doctl apps logs YOUR_APP_ID --type run --follow
```

### View Droplet logs

```bash
ssh root@YOUR_DROPLET_IP
docker logs mockforge -f
```

### View DOKS logs

```bash
kubectl logs -l app.kubernetes.io/name=mockforge --tail=100 -f
```

### Check App Platform status

```bash
doctl apps get YOUR_APP_ID
doctl apps list-deployments YOUR_APP_ID
```

## Backup and Recovery

### Droplet Snapshots

```bash
# Create snapshot
doctl compute droplet-action snapshot DROPLET_ID --snapshot-name mockforge-backup-$(date +%Y%m%d)

# Create from snapshot
doctl compute droplet create mockforge-restored \
  --image SNAPSHOT_ID \
  --region nyc1 \
  --size s-2vcpu-4gb
```

### Volume Backups

```bash
# Create volume
doctl compute volume create mockforge-data \
  --region nyc1 \
  --size 10GiB \
  --fs-type ext4

# Attach to droplet
doctl compute volume-action attach VOLUME_ID DROPLET_ID
```

## Additional Resources

- [DigitalOcean App Platform Documentation](https://docs.digitalocean.com/products/app-platform/)
- [DigitalOcean Kubernetes Documentation](https://docs.digitalocean.com/products/kubernetes/)
- [DigitalOcean Droplets Documentation](https://docs.digitalocean.com/products/droplets/)
- [doctl CLI Reference](https://docs.digitalocean.com/reference/doctl/)
