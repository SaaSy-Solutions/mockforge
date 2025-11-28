# MockForge Integration Ecosystem

A comprehensive guide to plugins, integrations, and community extensions that extend MockForge's capabilities.

## Overview

MockForge's extensible architecture enables integration with a wide variety of tools and services. This document catalogs available plugins, CI/CD integrations, cloud deployment options, monitoring tools, and SDKs that work with MockForge.

---

## 🔌 Plugin System

### Plugin Marketplace

The MockForge Plugin Marketplace provides a centralized repository for discovering, installing, and managing plugins.

**Location:** `/plugin-marketplace/`

**Features:**
- Plugin discovery and search
- Version management with dependency resolution
- Plugin publishing and sharing
- Reviews and ratings
- Author profiles and monetization
- Security scanning and validation

**Documentation:**
- [Plugin Marketplace README](../plugin-marketplace/README.md)
- [Plugin Registry Guide](PLUGIN_REGISTRY.md)
- [Plugin Marketplace Implementation](PLUGIN_MARKETPLACE_IMPLEMENTATION.md)

### Available Plugins

#### Official Plugins

| Plugin | Category | Description | Location |
|--------|----------|-------------|----------|
| **auth-jwt** | Authentication | JWT token validation and generation | `examples/plugins/auth-jwt/` |
| **auth-basic** | Authentication | HTTP Basic Authentication | `examples/plugins/auth-basic/` |
| **auth-python-oauth** | Authentication | OAuth 2.0 provider (Python-based) | `examples/plugins/auth-python-oauth/` |
| **template-crypto** | Templates | Cryptographic template functions | `examples/plugins/template-crypto/` |
| **template-custom** | Templates | Domain-specific data generation | `examples/plugins/template-custom/` |
| **template-fs** | Templates | File system template functions | `examples/plugins/template-fs/` |
| **datasource-csv** | Data Source | CSV file data source connector | `examples/plugins/datasource-csv/` |
| **response-graphql** | Response | GraphQL response generation | `examples/plugins/response-graphql/` |

**Full Plugin List:** See [Example Plugins README](../examples/plugins/README.md)

### Plugin Types

MockForge supports five plugin categories:

1. **Authentication Plugins** (`auth-*`)
   - Custom authentication schemes
   - JWT, OAuth2, SAML, Basic Auth
   - Session management

2. **Template Plugins** (`template-*`)
   - Custom template functions
   - Domain-specific data generators
   - Advanced formatting utilities

3. **Data Source Plugins** (`datasource-*`)
   - External data connectors
   - CSV, database, API integrations
   - Real-time data synchronization

4. **Response Plugins** (`response-*`)
   - Custom response generators
   - Protocol-specific transformers
   - Advanced data formatting

5. **Protocol Plugins** (`protocol-*`)
   - Custom protocol handlers
   - Protocol extensions
   - Legacy protocol support

### Installing Plugins

```bash
# Install from registry
mockforge plugin install auth-jwt

# Install specific version
mockforge plugin install auth-jwt@1.2.0

# Install from local directory
mockforge plugin install ./examples/plugins/auth-jwt/

# Install from Git repository
mockforge plugin install https://github.com/user/my-plugin

# Search for plugins
mockforge plugin search auth

# List installed plugins
mockforge plugin list
```

### Plugin Development

**Getting Started:**
- [Plugin Development Guide](../book/src/user-guide/plugins.md)
- [Plugin Template](../templates/plugin-template/)
- [Plugin Core API](../crates/mockforge-plugin-core/src/lib.rs)

**Resources:**
- [Plugin Quick Reference](plugins/QUICK_REFERENCE.md)
- [Polyglot Plugin Support](plugins/POLYGLOT_PLUGIN_SUPPORT.md)

---

## 🚀 CI/CD Integrations

### GitHub Actions

MockForge integrates seamlessly with GitHub Actions for automated testing and deployment.

**Example Workflow:**
```yaml
# .github/workflows/test.yml
name: Test with MockForge

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Start MockForge
        run: |
          cargo install mockforge-cli
          mockforge serve --spec api.yaml --http-port 3000 &
      - name: Run Tests
        run: npm test
```

**Available Workflows:**
- `.github/workflows/contract-validation.yml` - OpenAPI contract validation
- `.github/workflows/docker.yml` - Docker image builds
- `.github/workflows/ci.yml` - Continuous integration

**Documentation:**
- [Developer Workflow Integration](DEVELOPER_WORKFLOW_INTEGRATION.md)
- [Integration Automation Coverage](../INTEGRATION_AUTOMATION_COVERAGE.md)

### GitLab CI/CD

Full GitLab CI/CD pipeline support with multi-stage deployments.

**Example Pipeline:**
```yaml
# .gitlab-ci.yml
stages:
  - test
  - deploy

test:integration:
  stage: test
  script:
    - docker-compose up -d mockforge
    - npm test
```

**Features:**
- Integration testing with Docker Compose
- Staging and production deployments
- Kubernetes deployments
- Coverage reporting

**Documentation:**
- [GitLab CI Configuration](../.gitlab-ci.yml)

### Jenkins

Jenkins pipeline support for enterprise deployments.

**Example Jenkinsfile:**
```groovy
pipeline {
    agent any
    stages {
        stage('Test') {
            steps {
                sh 'docker-compose up -d'
                sh 'npm test'
            }
        }
        stage('Deploy') {
            steps {
                sh 'kubectl apply -f k8s/'
            }
        }
    }
}
```

**Documentation:**
- [Jenkinsfile](../Jenkinsfile)

### Test Framework Integrations

#### Playwright
```typescript
// examples/test-integration/playwright/
import { test } from '@playwright/test';
import { MockServer } from '@mockforge/sdk';

test.beforeEach(async () => {
  const server = await MockServer.start({ port: 3000 });
  // Use server in tests
});
```

#### Vitest
```typescript
// examples/test-integration/vitest/
import { describe, it, beforeAll } from 'vitest';
import { MockServer } from '@mockforge/sdk';

let server: MockServer;

beforeAll(async () => {
  server = await MockServer.start();
});
```

**Documentation:**
- [Test Integration Examples](../examples/test-integration/README.md)

---

## ☁️ Cloud Deployments

### AWS

**Services Supported:**
- ECS (Fargate) - Container orchestration
- EKS - Kubernetes on AWS
- App Runner - Serverless containers
- EC2 - Virtual machines
- Lambda - Serverless functions (via API Gateway)

**Documentation:**
- [AWS Deployment Guide](deployment/aws.md)

### Google Cloud Platform

**Services Supported:**
- Cloud Run - Serverless containers (Recommended)
- GKE - Google Kubernetes Engine
- Compute Engine - Virtual machines
- Cloud Functions - Serverless (via API Gateway)

**Documentation:**
- [GCP Deployment Guide](deployment/gcp.md)

### Microsoft Azure

**Services Supported:**
- Container Apps - Serverless containers
- ASK - Azure Kubernetes Service
- App Service - Managed app hosting
- Virtual Machines - Infrastructure

**Documentation:**
- [Azure Deployment Guide](deployment/azure.md)

### DigitalOcean

**Services Supported:**
- App Platform - Managed platform (Recommended)
- DOKS - DigitalOcean Kubernetes
- Droplets - Virtual machines

**Documentation:**
- [DigitalOcean Deployment Guide](deployment/digitalocean.md)

### Kubernetes

**Deployment Options:**
- Helm Charts - `helm/mockforge/`
- Raw Manifests - `k8s/`
- Operator - `crates/mockforge-k8s-operator/`

**Documentation:**
- [Kubernetes Deployment Guide](deployment/README.md)
- [K8s Operator README](../crates/mockforge-k8s-operator/README.md)

**Quick Start:**
```bash
# Using Helm
helm install mockforge ./helm/mockforge

# Using kubectl
kubectl apply -f k8s/
```

**Documentation:**
- [Cloud Deployment Guide](CLOUD_DEPLOYMENT.md)
- [Docker Deployment](../DOCKER.md)

---

## 🔐 Security & Authentication

### Mutual TLS (mTLS)

MockForge supports **Mutual TLS (mTLS)** for enhanced security:

```yaml
http:
  tls:
    enabled: true
    cert_file: "./certs/server.crt"
    key_file: "./certs/server.key"
    ca_file: "./certs/ca.crt"           # CA certificate for client verification
    require_client_cert: true            # Enable mTLS
```

**Documentation:**
- [mTLS Configuration Guide](mTLS_CONFIGURATION.md)

### Role-Based Access Control (RBAC)

Complete RBAC implementation with three roles (Admin, Editor, Viewer):

```yaml
collaboration:
  enabled: true
  roles:
    admin:
      permissions: ["*"]
    editor:
      permissions: ["mocks:*", "history:read"]
    viewer:
      permissions: ["mocks:read"]
```

**Documentation:**
- [RBAC Guide](RBAC_GUIDE.md)
- [Collaboration README](../crates/mockforge-collab/README.md)

### Audit Trails

Comprehensive audit logging for security and compliance:

- Authentication audit logs
- Request logging
- Collaboration history
- Configuration change tracking
- Plugin activity logs

**Documentation:**
- [Audit Trails Guide](AUDIT_TRAILS.md)

## 📊 Monitoring & Observability

### Integration Services

#### Slack
```yaml
integrations:
  slack:
    webhook_url: "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
    channel: "#mockforge-alerts"
    username: "MockForge"
    mention_users: ["U12345678"]
```

#### Microsoft Teams
```yaml
integrations:
  teams:
    webhook_url: "https://outlook.office.com/webhook/YOUR/WEBHOOK/URL"
    mention_users: ["user@company.com"]
```

#### Jira
```yaml
integrations:
  jira:
    url: "https://your-company.atlassian.net"
    username: "bot@company.com"
    api_token: "your-api-token"
    project_key: "OPS"
    issue_type: "Incident"
```

#### PagerDuty
```yaml
integrations:
  pagerduty:
    routing_key: "your-routing-key"
    severity: "error"
    dedup_key_prefix: "mockforge"
```

#### Grafana
```yaml
integrations:
  grafana:
    url: "https://grafana.company.com"
    api_key: "your-api-key"
    dashboard_uid: "mockforge-dashboard"
```

**Documentation:**
- [Advanced ML Features](ADVANCED_ML_FEATURES.md) - Integration configuration
- [Performance Monitoring](PERFORMANCE_MONITORING.md)

### Observability Tools

- **Prometheus** - Metrics export
- **OpenTelemetry** - Distributed tracing
- **Grafana** - Dashboards and visualization
- **Admin UI** - Built-in monitoring interface

**Documentation:**
- [Observability Crate](../crates/mockforge-observability/README.md)

---

## 💻 SDK Integrations

### Multi-Language SDKs

MockForge provides official SDKs for embedding mock servers in tests:

| Language | Package | Status | Documentation |
|----------|---------|---------|---------------|
| **Rust** | `mockforge-sdk` | ✅ Complete | [Rust SDK](../sdk/README.md#rust-sdk) |
| **Node.js/TypeScript** | `@mockforge/sdk` | ✅ Complete | [Node.js SDK](../sdk/README.md#nodejs-sdk) |
| **Python** | `mockforge-sdk` | ✅ Complete | [Python SDK](../sdk/README.md#python-sdk) |
| **Go** | `github.com/SaaSy-Solutions/mockforge/sdk/go` | ✅ Complete | [Go SDK](../sdk/README.md#go-sdk) |
| **Java** | `com.mockforge:mockforge-sdk` | ✅ Complete | [Java SDK](../sdk/java/README.md) |
| **.NET** | `MockForge.Sdk` | ✅ Complete | [.NET SDK](../sdk/dotnet/README.md) |

**Installation:**
```bash
# Rust
cargo add --dev mockforge-sdk

# Node.js
npm install @mockforge/sdk

# Python
pip install mockforge-sdk

# Go
go get github.com/SaaSy-Solutions/mockforge/sdk/go

# Java (Maven)
# Add to pom.xml:
# <dependency>
#   <groupId>com.mockforge</groupId>
#   <artifactId>mockforge-sdk</artifactId>
#   <version>0.1.0</version>
# </dependency>

# .NET
dotnet add package MockForge.Sdk
```

**Documentation:**
- [SDK README](../sdk/README.md)
- [SDK Examples](../examples/sdk-rust/)

**Features:**
- Embedded mock servers
- Programmatic stub configuration
- Template support
- Offline mode (no network dependencies)

---

## 🔧 Development Tools

### VS Code Extension

**Location:** `vscode-extension/`

**Features:**
- Syntax highlighting for MockForge configs
- IntelliSense for YAML/JSON configurations
- Live mock server status
- Quick actions for common tasks

**Documentation:**
- [VS Code Extension README](../vscode-extension/README.md)

### CLI Tools

**MockForge CLI** provides comprehensive command-line interface:

```bash
# Server management
mockforge serve --config config.yaml
mockforge tunnel --port 3000

# Plugin management
mockforge plugin install auth-jwt
mockforge plugin list

# Configuration
mockforge validate config.yaml
mockforge export --format yaml

# Registry operations
mockforge plugin registry search auth
mockforge plugin registry publish
```

**Documentation:**
- [CLI README](../crates/mockforge-cli/README.md)

---

## 🗄️ Database & Storage Integrations

### Data Source Plugins

- **CSV** - `datasource-csv` plugin
- **PostgreSQL** - Community plugin (planned)
- **Redis** - Community plugin (planned)
- **MongoDB** - Community plugin (planned)

**Configuration:**
```yaml
plugins:
  - name: datasource-csv
    config:
      csv_files:
        - name: "users"
          path: "data/users.csv"
```

---

## 🔐 Security Integrations

### Authentication Providers

- **JWT** - `auth-jwt` plugin
- **OAuth 2.0** - `auth-python-oauth` plugin
- **Basic Auth** - `auth-basic` plugin
- **SAML** - Community plugin (planned)

### Secret Management

- **HashCorp Vault** - Integration support
- **AWS Secrets Manager** - Via environment variables
- **Kubernetes Secrets** - Native K8s support

**Documentation:**
- [Vault Integration](VAULT_INTEGRATION.md)

---

## 📦 Container & Orchestration

### Docker

**Images:**
- Official Docker image (from Dockerfile)
- Multi-stage builds for optimization
- Docker Compose configurations

**Documentation:**
- [Docker Guide](../DOCKER.md)

### Docker Compose

**Configurations:**
- `docker-compose.yml` - Development
- `docker-compose.dev.yml` - Development with hot reload
- `docker-compose.microservices.yml` - Microservices testing
- `docker-compose.production.yml` - Production deployment

### Helm Charts

**Location:** `helm/mockforge/`

**Features:**
- Configurable values
- Multi-environment support
- Service mesh integration (Istio)

**Documentation:**
- [Helm Chart README](../helm/mockforge/README.md)

---

## 🔄 GitOps & Infrastructure

### ArgoCD

**Features:**
- GitOps deployment
- Configuration management
- Multi-cluster support

**Documentation:**
- [ArgoCD Integration](ARGOCD_GITOPS.md)
- [ArgoCD Application](../deploy/argocd/application.yaml)

### Terraform

Terraform modules for cloud deployments are available:
- AWS ECS/EKS modules
- GCP Cloud Run/GKE modules
- Azure Container Apps/AKS modules

---

## 📚 Additional Resources

### Documentation

- [Main Documentation](https://docs.mockforge.dev/)
- [API Reference](https://docs.rs/mockforge-core)
- [Examples Directory](../examples/)

### Community

- [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)
- [GitHub Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions)
- [Discord](https://discord.gg/2FxXqKpa) - Join our community chat
- [Contributing Guide](../CONTRIBUTING.md)

### Examples

- [Plugin Examples](../examples/plugins/)
- [SDK Examples](../examples/sdk-rust/)
- [Test Integration Examples](../examples/test-integration/)
- [Protocol Examples](../examples/protocols/)

---

## 🎯 Integration Roadmap

### Planned Integrations

- **Java SDK** - For JVM-based applications
- **.NET SDK** - For C#/.NET applications
- **PostgreSQL Data Source** - Database connector plugin
- **Redis Data Source** - Cache connector plugin
- **SAML Authentication** - Enterprise SSO plugin
- **Webhook Integrations** - Custom webhook support
- **API Gateway Plugins** - AWS API Gateway, Kong, etc.

### Contributing New Integrations

To add a new integration:

1. Create plugin or SDK following [Plugin Development Guide](../book/src/user-guide/plugins.md)
2. Add integration documentation
3. Submit PR with examples
4. Update this document

**Template:**
- [Plugin Template](../templates/plugin-template/)

---

## 📞 Support

For integration questions or issues:

- **Documentation**: Check relevant integration guides above
- **GitHub Issues**: [Report integration issues](https://github.com/SaaSy-Solutions/mockforge/issues)
- **GitHub Discussions**: [Ask integration questions](https://github.com/SaaSy-Solutions/mockforge/discussions)

---

**Last Updated:** 2025-01-27
**Version:** 1.0
