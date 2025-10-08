# HashiCorp Vault Integration

## Overview

MockForge integrates with HashiCorp Vault for secure secret management. This document describes the setup and usage of Vault integration.

## Architecture

MockForge supports two methods for Vault integration:

1. **Vault Agent Sidecar Injection** - Recommended for most use cases
2. **External Secrets Operator** - Alternative for GitOps workflows

## Prerequisites

- HashiCorp Vault installed in the cluster or accessible externally
- Vault Agent Injector installed (for sidecar method)
- External Secrets Operator installed (for ESO method)
- Kubernetes authentication method enabled in Vault

## Setup Instructions

### 1. Deploy Vault (if not already installed)

```bash
# Add HashiCorp Helm repo
helm repo add hashicorp https://helm.releases.hashicorp.com
helm repo update

# Install Vault
helm install vault hashicorp/vault \
  --namespace vault \
  --create-namespace \
  --set "server.ha.enabled=true" \
  --set "server.ha.replicas=3"

# Initialize and unseal Vault
kubectl exec -n vault vault-0 -- vault operator init
kubectl exec -n vault vault-0 -- vault operator unseal <key>
```

### 2. Configure Vault Authentication

```bash
# Apply the setup script
kubectl apply -f k8s/vault-integration.yaml

# Execute the setup script inside Vault
kubectl exec -n vault vault-0 -- /bin/sh /vault/config/vault-auth-setup.sh
```

### 3. Store Secrets in Vault

```bash
# Login to Vault
kubectl exec -n vault -it vault-0 -- /bin/sh
vault login

# Enable KV secrets engine
vault secrets enable -path=secret kv-v2

# Store MockForge secrets
vault kv put secret/mockforge/api-keys \
  slack_webhook_url="https://hooks.slack.com/services/YOUR/WEBHOOK/URL" \
  pagerduty_service_key="YOUR_PAGERDUTY_KEY" \
  smtp_username="alerts@example.com" \
  smtp_password="YOUR_SMTP_PASSWORD"

# Store API keys
vault kv put secret/mockforge/config \
  api_keys='["key1", "key2", "key3"]'
```

### 4. Deploy MockForge with Vault Integration

#### Option A: Vault Agent Sidecar Injection

```bash
# Deploy with Vault annotations
kubectl apply -f k8s/vault-integration.yaml
```

The Vault Agent will automatically:
- Authenticate using the Kubernetes service account
- Fetch secrets from Vault
- Render templates to `/vault/secrets/`
- Keep secrets up-to-date

#### Option B: External Secrets Operator

```bash
# Install External Secrets Operator
helm repo add external-secrets https://charts.external-secrets.io
helm install external-secrets \
  external-secrets/external-secrets \
  -n external-secrets-system \
  --create-namespace

# Apply SecretStore and ExternalSecret
kubectl apply -f k8s/vault-integration.yaml
```

## Accessing Secrets in MockForge

### Environment Variables

Secrets are injected as environment variables:

```rust
use std::env;

let slack_webhook = env::var("SLACK_WEBHOOK_URL")
    .expect("SLACK_WEBHOOK_URL must be set");
let pagerduty_key = env::var("PAGERDUTY_SERVICE_KEY")
    .expect("PAGERDUTY_SERVICE_KEY must be set");
```

### File-based Secrets

Secrets are also available as files in `/vault/secrets/`:

```rust
use std::fs;

let api_keys = fs::read_to_string("/vault/secrets/config")
    .expect("Failed to read Vault secrets");
```

## Secret Rotation

### Automatic Rotation

Vault Agent automatically rotates secrets based on the TTL:

```hcl
template {
  source      = "/vault/configs/database.tmpl"
  destination = "/vault/secrets/database"
}
```

### Manual Rotation

To manually rotate a secret:

```bash
# Update secret in Vault
vault kv put secret/mockforge/api-keys \
  slack_webhook_url="https://new-webhook-url"

# Vault Agent will automatically detect and update within 5 minutes
# Or force restart the pod
kubectl rollout restart deployment/mockforge -n mockforge
```

## TLS Certificate Management

Vault can automatically issue and renew TLS certificates:

```bash
# Enable PKI secrets engine
vault secrets enable pki

# Configure PKI
vault secrets tune -max-lease-ttl=87600h pki
vault write pki/root/generate/internal \
  common_name=mockforge.io \
  ttl=87600h

# Create role
vault write pki/roles/mockforge \
  allowed_domains=mockforge.example.com \
  allow_subdomains=true \
  max_ttl=72h
```

Certificates are automatically renewed before expiry.

## Monitoring

### Vault Metrics

Monitor Vault integration health:

```promql
# Vault authentication failures
vault_auth_failure_total

# Secret fetch failures
vault_secret_fetch_errors_total

# Token expiration time
vault_token_ttl_seconds
```

### Vault Agent Logs

Check Vault Agent logs:

```bash
kubectl logs -n mockforge deployment/mockforge -c vault-agent
```

## Troubleshooting

### Secret Not Found

```bash
# Verify secret exists in Vault
vault kv get secret/mockforge/api-keys

# Check Vault policy
vault policy read mockforge

# Verify service account can authenticate
kubectl exec -n mockforge deployment/mockforge -c vault-agent -- cat /vault/secrets/.vault-token
```

### Authentication Failures

```bash
# Verify Kubernetes auth is configured
vault read auth/kubernetes/config

# Check role binding
vault read auth/kubernetes/role/mockforge

# Verify service account
kubectl get sa -n mockforge mockforge-vault
```

### Permission Denied

```bash
# Review Vault policy
vault policy read mockforge

# Test policy
vault token create -policy=mockforge
vault login <token>
vault kv get secret/mockforge/api-keys
```

## Security Best Practices

1. **Principle of Least Privilege**: Grant only required permissions
2. **Short TTLs**: Use short token TTLs and enable auto-renewal
3. **Audit Logging**: Enable Vault audit logging
4. **Network Policies**: Restrict network access to Vault
5. **Encryption**: Use TLS for all Vault communication
6. **Rotation**: Regularly rotate all secrets
7. **Monitoring**: Monitor Vault metrics and logs for anomalies

## References

- [Vault Kubernetes Auth](https://www.vaultproject.io/docs/auth/kubernetes)
- [Vault Agent](https://www.vaultproject.io/docs/agent)
- [External Secrets Operator](https://external-secrets.io/)
- [Vault Security Model](https://www.vaultproject.io/docs/internals/security)
