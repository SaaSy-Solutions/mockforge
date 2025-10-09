# Deploying MockForge on Microsoft Azure

This guide covers deploying MockForge on Microsoft Azure using various services.

## Table of Contents

- [Azure Container Instances (ACI)](#azure-container-instances-aci)
- [Azure Kubernetes Service (AKS)](#azure-kubernetes-service-aks)
- [Azure Container Apps](#azure-container-apps-recommended)
- [Azure Virtual Machines](#azure-virtual-machines)
- [Cost Estimation](#cost-estimation)

## Azure Container Apps (Recommended)

Azure Container Apps is the easiest way to deploy containerized applications on Azure.

### Prerequisites

- Azure CLI installed
- An Azure subscription

### Step 1: Install Container Apps Extension

```bash
az extension add --name containerapp --upgrade
az provider register --namespace Microsoft.App
az provider register --namespace Microsoft.OperationalInsights
```

### Step 2: Create Resource Group

```bash
az group create \
  --name mockforge-rg \
  --location eastus
```

### Step 3: Create Container Apps Environment

```bash
az containerapp env create \
  --name mockforge-env \
  --resource-group mockforge-rg \
  --location eastus
```

### Step 4: Deploy MockForge

```bash
az containerapp create \
  --name mockforge \
  --resource-group mockforge-rg \
  --environment mockforge-env \
  --image ghcr.io/saasy-solutions/mockforge:latest \
  --target-port 3000 \
  --ingress external \
  --cpu 1 \
  --memory 2Gi \
  --min-replicas 1 \
  --max-replicas 10 \
  --env-vars \
    MOCKFORGE_HTTP_PORT=3000 \
    MOCKFORGE_ADMIN_ENABLED=true \
    MOCKFORGE_ADMIN_PORT=9080 \
    RUST_LOG=info
```

### Step 5: Get Application URL

```bash
az containerapp show \
  --name mockforge \
  --resource-group mockforge-rg \
  --query properties.configuration.ingress.fqdn \
  --output tsv
```

### Using ARM Template

Create `template.json`:

```json
{
  "$schema": "https://schema.management.azure.com/schemas/2019-04-01/deploymentTemplate.json#",
  "contentVersion": "1.0.0.0",
  "parameters": {
    "containerAppName": {
      "type": "string",
      "defaultValue": "mockforge"
    },
    "location": {
      "type": "string",
      "defaultValue": "[resourceGroup().location]"
    }
  },
  "resources": [
    {
      "type": "Microsoft.App/managedEnvironments",
      "apiVersion": "2022-03-01",
      "name": "[concat(parameters('containerAppName'), '-env')]",
      "location": "[parameters('location')]",
      "properties": {
        "appLogsConfiguration": {
          "destination": "log-analytics"
        }
      }
    },
    {
      "type": "Microsoft.App/containerApps",
      "apiVersion": "2022-03-01",
      "name": "[parameters('containerAppName')]",
      "location": "[parameters('location')]",
      "dependsOn": [
        "[resourceId('Microsoft.App/managedEnvironments', concat(parameters('containerAppName'), '-env'))]"
      ],
      "properties": {
        "managedEnvironmentId": "[resourceId('Microsoft.App/managedEnvironments', concat(parameters('containerAppName'), '-env'))]",
        "configuration": {
          "ingress": {
            "external": true,
            "targetPort": 3000
          }
        },
        "template": {
          "containers": [
            {
              "name": "mockforge",
              "image": "ghcr.io/saasy-solutions/mockforge:latest",
              "resources": {
                "cpu": 1,
                "memory": "2Gi"
              },
              "env": [
                {
                  "name": "MOCKFORGE_HTTP_PORT",
                  "value": "3000"
                },
                {
                  "name": "MOCKFORGE_ADMIN_ENABLED",
                  "value": "true"
                }
              ]
            }
          ],
          "scale": {
            "minReplicas": 1,
            "maxReplicas": 10
          }
        }
      }
    }
  ],
  "outputs": {
    "fqdn": {
      "type": "string",
      "value": "[reference(resourceId('Microsoft.App/containerApps', parameters('containerAppName'))).configuration.ingress.fqdn]"
    }
  }
}
```

Deploy with:

```bash
az deployment group create \
  --resource-group mockforge-rg \
  --template-file template.json
```

## Azure Container Instances (ACI)

For simple single-container deployments.

### Step 1: Create Container Instance

```bash
az container create \
  --resource-group mockforge-rg \
  --name mockforge \
  --image ghcr.io/saasy-solutions/mockforge:latest \
  --cpu 1 \
  --memory 2 \
  --ports 3000 9080 \
  --dns-name-label mockforge-demo \
  --environment-variables \
    MOCKFORGE_HTTP_PORT=3000 \
    MOCKFORGE_ADMIN_ENABLED=true \
    MOCKFORGE_ADMIN_PORT=9080 \
  --location eastus
```

### Step 2: Get Container FQDN

```bash
az container show \
  --resource-group mockforge-rg \
  --name mockforge \
  --query ipAddress.fqdn \
  --output tsv
```

### Using YAML Deployment

Create `aci-deployment.yaml`:

```yaml
apiVersion: '2019-12-01'
location: eastus
name: mockforge
properties:
  containers:
  - name: mockforge
    properties:
      image: ghcr.io/saasy-solutions/mockforge:latest
      resources:
        requests:
          cpu: 1
          memoryInGb: 2
      ports:
      - port: 3000
        protocol: TCP
      - port: 9080
        protocol: TCP
      environmentVariables:
      - name: MOCKFORGE_HTTP_PORT
        value: '3000'
      - name: MOCKFORGE_ADMIN_ENABLED
        value: 'true'
      - name: MOCKFORGE_ADMIN_PORT
        value: '9080'
  osType: Linux
  ipAddress:
    type: Public
    ports:
    - protocol: TCP
      port: 3000
    - protocol: TCP
      port: 9080
    dnsNameLabel: mockforge-demo
  restartPolicy: Always
tags: {}
type: Microsoft.ContainerInstance/containerGroups
```

Deploy with:

```bash
az container create \
  --resource-group mockforge-rg \
  --file aci-deployment.yaml
```

## Azure Kubernetes Service (AKS)

For production workloads requiring orchestration.

### Step 1: Create AKS Cluster

```bash
az aks create \
  --resource-group mockforge-rg \
  --name mockforge-cluster \
  --node-count 3 \
  --node-vm-size Standard_D2s_v3 \
  --enable-addons monitoring \
  --enable-managed-identity \
  --enable-cluster-autoscaler \
  --min-count 3 \
  --max-count 10 \
  --location eastus
```

### Step 2: Get Credentials

```bash
az aks get-credentials \
  --resource-group mockforge-rg \
  --name mockforge-cluster
```

### Step 3: Deploy with Helm

```bash
# Install from local chart
helm install mockforge ./helm/mockforge \
  --set image.repository=ghcr.io/saasy-solutions/mockforge \
  --set image.tag=latest \
  --set ingress.enabled=true \
  --set ingress.className=azure

# Or from repository (when published)
helm repo add mockforge https://charts.mockforge.dev
helm install mockforge mockforge/mockforge
```

### Step 4: Set up Azure Application Gateway Ingress

```bash
# Install AGIC
az aks enable-addons \
  --resource-group mockforge-rg \
  --name mockforge-cluster \
  --addons ingress-appgw \
  --appgw-name mockforge-appgw \
  --appgw-subnet-cidr 10.2.0.0/16
```

## Azure Virtual Machines

Traditional VM deployment with Azure.

### Step 1: Create VM

```bash
az vm create \
  --resource-group mockforge-rg \
  --name mockforge-vm \
  --image Ubuntu2204 \
  --size Standard_B2s \
  --admin-username azureuser \
  --generate-ssh-keys \
  --custom-data cloud-init.txt
```

### Step 2: Cloud-Init Script

Create `cloud-init.txt`:

```yaml
#cloud-config
package_upgrade: true
packages:
  - docker.io

runcmd:
  - systemctl start docker
  - systemctl enable docker
  - docker pull ghcr.io/saasy-solutions/mockforge:latest
  - docker run -d --name mockforge --restart unless-stopped -p 80:3000 -p 9080:9080 -e MOCKFORGE_HTTP_PORT=3000 -e MOCKFORGE_ADMIN_ENABLED=true ghcr.io/saasy-solutions/mockforge:latest
```

### Step 3: Open Ports

```bash
az vm open-port \
  --resource-group mockforge-rg \
  --name mockforge-vm \
  --port 80 \
  --priority 1001

az vm open-port \
  --resource-group mockforge-rg \
  --name mockforge-vm \
  --port 9080 \
  --priority 1002
```

### Step 4: Create VM Scale Set

```bash
az vmss create \
  --resource-group mockforge-rg \
  --name mockforge-vmss \
  --image Ubuntu2204 \
  --instance-count 3 \
  --vm-sku Standard_B2s \
  --admin-username azureuser \
  --generate-ssh-keys \
  --custom-data cloud-init.txt \
  --load-balancer mockforge-lb \
  --backend-pool-name mockforge-pool

# Configure autoscaling
az monitor autoscale create \
  --resource-group mockforge-rg \
  --resource mockforge-vmss \
  --resource-type Microsoft.Compute/virtualMachineScaleSets \
  --name mockforge-autoscale \
  --min-count 3 \
  --max-count 10 \
  --count 3

az monitor autoscale rule create \
  --resource-group mockforge-rg \
  --autoscale-name mockforge-autoscale \
  --condition "Percentage CPU > 70 avg 5m" \
  --scale out 1
```

## Using Azure Container Registry

For private images:

```bash
# Create ACR
az acr create \
  --resource-group mockforge-rg \
  --name mockforgeacr \
  --sku Basic

# Log in to ACR
az acr login --name mockforgeacr

# Tag and push
docker tag ghcr.io/saasy-solutions/mockforge:latest \
  mockforgeacr.azurecr.io/mockforge:latest

docker push mockforgeacr.azurecr.io/mockforge:latest

# Grant AKS access to ACR
az aks update \
  --resource-group mockforge-rg \
  --name mockforge-cluster \
  --attach-acr mockforgeacr
```

## Cost Estimation

### Container Apps (1 vCPU, 2 GB)
- **Compute:** ~$0.000012/vCPU-second, ~$0.0000014/GB-second
- **Estimated:** $15-35/month (depending on usage)
- **Cheapest option for most use cases**

### Container Instances (1 vCPU, 2 GB, always running)
- **Compute:** ~$0.0000125/vCPU-second, ~$0.0000014/GB-second
- **Estimated:** ~$45/month

### AKS (3 nodes, Standard_D2s_v3)
- **Cluster management:** Free
- **Worker nodes:** ~$70/month per node = $210/month
- **Load balancer:** ~$18/month
- **Total:** ~$228/month

### VM Scale Set (3 × Standard_B2s)
- **VMs:** ~$30/month per VM = $90/month
- **Load balancer:** ~$18/month
- **Storage:** ~$10/month
- **Total:** ~$118/month

## Best Practices

1. **Use Container Apps** for most scenarios - best price/performance
2. **Enable Azure Monitor** for comprehensive monitoring
3. **Use Azure Key Vault** for secrets management
4. **Implement Azure Front Door** for global load balancing
5. **Enable Azure Policy** for governance
6. **Use Azure Private Link** for secure connectivity
7. **Implement Azure AD** for authentication

## Monitoring with Azure

### Azure Monitor

```bash
# View logs
az monitor log-analytics query \
  --workspace <workspace-id> \
  --analytics-query "ContainerAppConsoleLogs_CL | where ContainerAppName_s == 'mockforge' | top 50 by TimeGenerated"
```

### Application Insights

Enable Application Insights for Container Apps:

```bash
az containerapp update \
  --name mockforge \
  --resource-group mockforge-rg \
  --set-env-vars \
    APPLICATIONINSIGHTS_CONNECTION_STRING=<connection-string>
```

### Alerts

```bash
# Create metric alert
az monitor metrics alert create \
  --name mockforge-high-cpu \
  --resource-group mockforge-rg \
  --scopes /subscriptions/<sub-id>/resourceGroups/mockforge-rg/providers/Microsoft.App/containerApps/mockforge \
  --condition "avg Percentage CPU > 80" \
  --description "Alert when CPU exceeds 80%"
```

## Troubleshooting

### View Container App logs

```bash
az containerapp logs show \
  --name mockforge \
  --resource-group mockforge-rg \
  --follow
```

### View Container Instance logs

```bash
az container logs \
  --resource-group mockforge-rg \
  --name mockforge \
  --follow
```

### Debug AKS pods

```bash
kubectl get pods -l app.kubernetes.io/name=mockforge
kubectl logs -l app.kubernetes.io/name=mockforge --tail=100
kubectl describe pod <pod-name>
```

## Additional Resources

- [Azure Container Apps Documentation](https://docs.microsoft.com/azure/container-apps/)
- [Azure Container Instances Documentation](https://docs.microsoft.com/azure/container-instances/)
- [Azure Kubernetes Service Documentation](https://docs.microsoft.com/azure/aks/)
- [Azure Resource Manager Templates](https://docs.microsoft.com/azure/azure-resource-manager/templates/)
