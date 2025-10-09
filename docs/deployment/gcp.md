# Deploying MockForge on Google Cloud Platform

This guide covers deploying MockForge on Google Cloud Platform using various services.

## Table of Contents

- [Google Cloud Run](#google-cloud-run-recommended)
- [Google Kubernetes Engine (GKE)](#google-kubernetes-engine-gke)
- [Google Compute Engine](#google-compute-engine)
- [Cost Estimation](#cost-estimation)

## Google Cloud Run (Recommended)

Cloud Run is the easiest and most cost-effective way to deploy MockForge on GCP.

### Prerequisites

- Google Cloud SDK (`gcloud`) installed
- A GCP project with billing enabled

### Step 1: Enable Required APIs

```bash
gcloud services enable run.googleapis.com
gcloud services enable containerregistry.googleapis.com
```

### Step 2: Deploy to Cloud Run

```bash
# Deploy from public image
gcloud run deploy mockforge \
  --image ghcr.io/saasy-solutions/mockforge:latest \
  --platform managed \
  --region us-central1 \
  --port 3000 \
  --allow-unauthenticated \
  --set-env-vars "MOCKFORGE_HTTP_PORT=3000,MOCKFORGE_ADMIN_ENABLED=true,MOCKFORGE_ADMIN_PORT=9080" \
  --memory 512Mi \
  --cpu 1 \
  --min-instances 1 \
  --max-instances 10 \
  --concurrency 80
```

### Step 3: Get Service URL

```bash
gcloud run services describe mockforge \
  --platform managed \
  --region us-central1 \
  --format 'value(status.url)'
```

### Using Cloud Run with Custom Domain

```bash
# Map custom domain
gcloud run domain-mappings create \
  --service mockforge \
  --domain api.example.com \
  --region us-central1

# Verify domain ownership in Google Search Console
```

### Infrastructure as Code (Terraform)

Create `main.tf`:

```hcl
terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

variable "project_id" {
  description = "GCP Project ID"
  type        = string
}

variable "region" {
  description = "GCP Region"
  type        = string
  default     = "us-central1"
}

resource "google_cloud_run_service" "mockforge" {
  name     = "mockforge"
  location = var.region

  template {
    spec {
      containers {
        image = "ghcr.io/saasy-solutions/mockforge:latest"

        ports {
          container_port = 3000
        }

        env {
          name  = "MOCKFORGE_HTTP_PORT"
          value = "3000"
        }

        env {
          name  = "MOCKFORGE_ADMIN_ENABLED"
          value = "true"
        }

        env {
          name  = "MOCKFORGE_ADMIN_PORT"
          value = "9080"
        }

        resources {
          limits = {
            cpu    = "1"
            memory = "512Mi"
          }
        }
      }

      container_concurrency = 80
    }

    metadata {
      annotations = {
        "autoscaling.knative.dev/minScale" = "1"
        "autoscaling.knative.dev/maxScale" = "10"
      }
    }
  }

  traffic {
    percent         = 100
    latest_revision = true
  }
}

resource "google_cloud_run_service_iam_member" "public" {
  service  = google_cloud_run_service.mockforge.name
  location = google_cloud_run_service.mockforge.location
  role     = "roles/run.invoker"
  member   = "allUsers"
}

output "url" {
  value = google_cloud_run_service.mockforge.status[0].url
}
```

Deploy with:

```bash
terraform init
terraform plan -var="project_id=my-project-id"
terraform apply -var="project_id=my-project-id"
```

## Google Kubernetes Engine (GKE)

Deploy MockForge on GKE for production workloads requiring more control.

### Step 1: Create GKE Cluster

```bash
gcloud container clusters create mockforge-cluster \
  --region us-central1 \
  --num-nodes 3 \
  --machine-type n1-standard-2 \
  --enable-autoscaling \
  --min-nodes 3 \
  --max-nodes 10 \
  --enable-autorepair \
  --enable-autoupgrade
```

### Step 2: Get Credentials

```bash
gcloud container clusters get-credentials mockforge-cluster \
  --region us-central1
```

### Step 3: Deploy with Helm

```bash
# Install from local chart
helm install mockforge ./helm/mockforge \
  --set image.repository=ghcr.io/saasy-solutions/mockforge \
  --set image.tag=latest \
  --set ingress.enabled=true \
  --set ingress.className=gce

# Or from repository (when published)
helm repo add mockforge https://charts.mockforge.dev
helm install mockforge mockforge/mockforge
```

### Step 4: Set up Google Cloud Load Balancer

```bash
# Install ingress-gce (usually pre-installed on GKE)
kubectl apply -f - <<EOF
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: mockforge-ingress
  annotations:
    kubernetes.io/ingress.class: "gce"
    kubernetes.io/ingress.global-static-ip-name: "mockforge-ip"
spec:
  rules:
  - host: api.example.com
    http:
      paths:
      - path: /*
        pathType: ImplementationSpecific
        backend:
          service:
            name: mockforge
            port:
              number: 3000
EOF
```

### Step 5: Reserve Static IP

```bash
gcloud compute addresses create mockforge-ip \
  --global

gcloud compute addresses describe mockforge-ip \
  --global \
  --format="value(address)"
```

## Google Compute Engine

Deploy on VM instances for traditional deployments.

### Step 1: Create VM Instance

```bash
gcloud compute instances create mockforge-vm \
  --zone us-central1-a \
  --machine-type e2-medium \
  --image-family ubuntu-2204-lts \
  --image-project ubuntu-os-cloud \
  --boot-disk-size 20GB \
  --tags http-server,https-server \
  --metadata-from-file startup-script=startup.sh
```

### Step 2: Startup Script

Create `startup.sh`:

```bash
#!/bin/bash

# Install Docker
apt-get update
apt-get install -y docker.io
systemctl start docker
systemctl enable docker

# Pull and run MockForge
docker pull ghcr.io/saasy-solutions/mockforge:latest

docker run -d \
  --name mockforge \
  --restart unless-stopped \
  -p 80:3000 \
  -p 9080:9080 \
  -e MOCKFORGE_HTTP_PORT=3000 \
  -e MOCKFORGE_ADMIN_PORT=9080 \
  -e MOCKFORGE_ADMIN_ENABLED=true \
  -e RUST_LOG=info \
  ghcr.io/saasy-solutions/mockforge:latest

# Configure firewall
ufw allow 80/tcp
ufw allow 9080/tcp
ufw --force enable
```

### Step 3: Create Firewall Rules

```bash
gcloud compute firewall-rules create allow-mockforge-http \
  --allow tcp:80 \
  --source-ranges 0.0.0.0/0 \
  --target-tags http-server

gcloud compute firewall-rules create allow-mockforge-admin \
  --allow tcp:9080 \
  --source-ranges YOUR_IP/32 \
  --target-tags http-server
```

### Step 4: Create Instance Template and Managed Instance Group

```bash
# Create instance template
gcloud compute instance-templates create mockforge-template \
  --machine-type e2-medium \
  --image-family ubuntu-2204-lts \
  --image-project ubuntu-os-cloud \
  --boot-disk-size 20GB \
  --tags http-server \
  --metadata-from-file startup-script=startup.sh

# Create managed instance group
gcloud compute instance-groups managed create mockforge-group \
  --base-instance-name mockforge \
  --template mockforge-template \
  --size 3 \
  --zone us-central1-a

# Set up autoscaling
gcloud compute instance-groups managed set-autoscaling mockforge-group \
  --zone us-central1-a \
  --min-num-replicas 3 \
  --max-num-replicas 10 \
  --target-cpu-utilization 0.7

# Create health check
gcloud compute health-checks create http mockforge-health \
  --port 3000 \
  --request-path /ping

# Set named port
gcloud compute instance-groups managed set-named-ports mockforge-group \
  --named-ports http:3000 \
  --zone us-central1-a
```

### Step 5: Create Load Balancer

```bash
# Create backend service
gcloud compute backend-services create mockforge-backend \
  --protocol HTTP \
  --health-checks mockforge-health \
  --global

# Add instance group to backend
gcloud compute backend-services add-backend mockforge-backend \
  --instance-group mockforge-group \
  --instance-group-zone us-central1-a \
  --global

# Create URL map
gcloud compute url-maps create mockforge-lb \
  --default-service mockforge-backend

# Create target HTTP proxy
gcloud compute target-http-proxies create mockforge-proxy \
  --url-map mockforge-lb

# Create forwarding rule
gcloud compute forwarding-rules create mockforge-rule \
  --global \
  --target-http-proxy mockforge-proxy \
  --ports 80
```

## Using Google Artifact Registry

For private images:

```bash
# Enable Artifact Registry
gcloud services enable artifactregistry.googleapis.com

# Create repository
gcloud artifacts repositories create mockforge \
  --repository-format docker \
  --location us-central1

# Configure Docker auth
gcloud auth configure-docker us-central1-docker.pkg.dev

# Tag and push
docker tag ghcr.io/saasy-solutions/mockforge:latest \
  us-central1-docker.pkg.dev/PROJECT_ID/mockforge/mockforge:latest

docker push us-central1-docker.pkg.dev/PROJECT_ID/mockforge/mockforge:latest
```

## Cost Estimation

### Cloud Run (1 vCPU, 512 MB, 80 concurrent)
- **Requests:** Free tier covers ~2M requests/month
- **CPU time:** ~$0.00002400/vCPU-second
- **Memory:** ~$0.00000250/GiB-second
- **Estimated:** $10-30/month (depending on traffic)

### GKE (3 nodes, n1-standard-2)
- **Cluster management:** Free for Autopilot, $73/month for Standard
- **Worker nodes:** ~$50/month per node = $150/month
- **Load balancer:** ~$18/month
- **Total:** ~$168-240/month

### Compute Engine (3 × e2-medium)
- **Instances:** ~$25/month per instance = $75/month
- **Load balancer:** ~$18/month
- **Persistent disk:** ~$10/month
- **Total:** ~$103/month

## Best Practices

1. **Use Cloud Run** for most use cases - it's the easiest and cheapest
2. **Enable Cloud Logging** for centralized logs
3. **Use Cloud Monitoring** for metrics and alerting
4. **Store secrets in Secret Manager**, not environment variables
5. **Use VPC Service Controls** for enhanced security
6. **Enable Cloud Armor** for DDoS protection
7. **Use Cloud CDN** for static content

## Monitoring with Google Cloud

### Cloud Logging

```bash
# View logs
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=mockforge" \
  --limit 50 \
  --format json
```

### Cloud Monitoring

```bash
# Create uptime check
gcloud monitoring uptime create mockforge-uptime \
  --display-name="MockForge Uptime" \
  --resource-type=uptime-url \
  --http-check-path=/ping \
  --monitored-resource-label=host=YOUR_CLOUD_RUN_URL
```

### Cloud Trace

Cloud Run automatically integrates with Cloud Trace for distributed tracing.

## Troubleshooting

### View Cloud Run logs

```bash
gcloud logging tail "resource.type=cloud_run_revision" \
  --filter="resource.labels.service_name=mockforge"
```

### Check Cloud Run service status

```bash
gcloud run services describe mockforge \
  --platform managed \
  --region us-central1
```

### Debug GKE pods

```bash
kubectl get pods -l app.kubernetes.io/name=mockforge
kubectl logs -l app.kubernetes.io/name=mockforge --tail=100
kubectl describe pod <pod-name>
```

## Additional Resources

- [Google Cloud Run Documentation](https://cloud.google.com/run/docs)
- [Google Kubernetes Engine Documentation](https://cloud.google.com/kubernetes-engine/docs)
- [Google Compute Engine Documentation](https://cloud.google.com/compute/docs)
- [Terraform Google Provider](https://registry.terraform.io/providers/hashicorp/google/latest/docs)
