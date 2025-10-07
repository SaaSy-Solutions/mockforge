# Developer Workflow & Tooling Integration - Complete Guide

## 🎯 Overview

This guide covers all the developer workflow integrations we've built for MockForge, positioning it as the backbone of your development pipeline.

## Table of Contents

1. [VS Code Extension](#vs-code-extension)
2. [API Collection Integration](#api-collection-integration)
3. [Docker Compose Automation](#docker-compose-automation)
4. [CI/CD Validation](#cicd-validation)

---

## 📦 VS Code Extension

### Installation

```bash
cd vscode-extension
npm install
npm run compile
```

### Features

#### 1. Mocks Explorer
- Visual tree view of all active mocks
- Real-time WebSocket updates when mocks change
- Context menu actions (edit, delete, toggle)
- Color-coded by HTTP method

#### 2. Server Control Panel
- View server status, version, and uptime
- Monitor total requests and active mocks
- Quick access to server statistics

#### 3. Mock Management
- **Create Mocks**: Interactive wizard for creating new mocks
- **Edit Mocks**: JSON editor with syntax highlighting
- **Export/Import**: Save and restore mock configurations
- **Toggle Mocks**: Enable/disable mocks with a click

### Configuration

```json
{
  "mockforge.serverUrl": "http://localhost:3000",
  "mockforge.autoConnect": true,
  "mockforge.showNotifications": true
}
```

### API Endpoints Used

- `GET /__mockforge/api/mocks` - List all mocks
- `POST /__mockforge/api/mocks` - Create mock
- `PUT /__mockforge/api/mocks/:id` - Update mock
- `DELETE /__mockforge/api/mocks/:id` - Delete mock
- `GET /__mockforge/api/stats` - Server statistics
- `WS /__mockforge/ws` - Real-time updates

### Development

```bash
# Watch mode for development
npm run watch

# Debug extension
# Press F5 in VS Code to launch Extension Development Host
```

---

## 🔄 API Collection Integration

### Supported Formats

- **Postman** (v2.1) - Most popular API client
- **Insomnia** (v4) - Developer-friendly API client
- **Hoppscotch** - Open-source API development

### Programmatic Usage

```rust
use mockforge_core::collection_export::{CollectionExporter, CollectionFormat};
use mockforge_core::openapi::OpenApiSpec;

// Load your OpenAPI spec
let spec = OpenApiSpec::from_file("api.yaml").await?;

// Create exporter
let exporter = CollectionExporter::new("http://localhost:3000".to_string());

// Generate Postman collection
let postman_collection = exporter.to_postman(&spec);
let json = serde_json::to_string_pretty(&postman_collection)?;
std::fs::write("collection.json", json)?;

// Generate Insomnia workspace
let insomnia_collection = exporter.to_insomnia(&spec);
let json = serde_json::to_string_pretty(&insomnia_collection)?;

// Generate Hoppscotch collection
let hoppscotch_collection = exporter.to_hoppscotch(&spec);
```

### CLI Usage (Future)

```bash
# Export to Postman
mockforge export --spec api.yaml --format postman --output collection.json

# Export to Insomnia
mockforge export --spec api.yaml --format insomnia --output workspace.json

# Export to Hoppscotch
mockforge export --spec api.yaml --format hoppscotch --output collection.json
```

### Collection Features

Each generated collection includes:
- ✅ All endpoints from your OpenAPI spec
- ✅ Proper HTTP methods and paths
- ✅ Example request bodies for POST/PUT/PATCH
- ✅ Base URL as environment variable
- ✅ Content-Type headers pre-configured

### Bi-directional Sync

Import collections from Postman/Insomnia and auto-generate mocks:

```typescript
// VS Code Extension feature
await client.importMocks(collectionData, 'postman', true);
```

---

## 🐳 Docker Compose Automation

### Quick Start

```bash
# Use provided microservices setup
docker-compose -f docker-compose.microservices.yml up
```

### Programmatic Generation

```rust
use mockforge_core::docker_compose::{
    DockerComposeGenerator,
    MockServiceSpec,
};
use std::collections::HashMap;

let generator = DockerComposeGenerator::new("mockforge-network".to_string())
    .with_image("mockforge:latest".to_string());

// Define your services
let services = vec![
    MockServiceSpec {
        name: "auth".to_string(),
        port: 3001,
        spec_path: Some("auth.yaml".to_string()),
        config_path: None,
    },
    MockServiceSpec {
        name: "users".to_string(),
        port: 3002,
        spec_path: Some("users.yaml".to_string()),
        config_path: None,
    },
];

// Add dependencies
let mut deps = HashMap::new();
deps.insert("users".to_string(), vec!["auth".to_string()]);

// Generate docker-compose
let config = generator.generate_with_dependencies(services, deps);
let yaml = generator.to_yaml(&config)?;

std::fs::write("docker-compose.yml", yaml)?;
```

### Features

- **Health Checks**: Automatic endpoint health checking
- **Service Dependencies**: Proper startup ordering
- **Network Isolation**: All services on dedicated network
- **Volume Mounts**: Specs, configs, and logs
- **Environment Variables**: Full configuration control

### Directory Structure

```
project/
├── docker-compose.yml
├── specs/
│   ├── auth.yaml
│   ├── users.yaml
│   └── orders.yaml
├── configs/
│   └── auth-config.yaml
└── logs/
```

### Testing Locally

```bash
# Start all services
docker-compose up -d

# Check health
for port in 3001 3002 3003 3004; do
  curl http://localhost:$port/health
done

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

See [`docs/DOCKER_COMPOSE_GUIDE.md`](DOCKER_COMPOSE_GUIDE.md) for detailed documentation.

---

## ⚙️ CI/CD Validation

### Contract Validation

Ensure your mocks match real API behavior:

```rust
use mockforge_core::contract_validation::ContractValidator;

let validator = ContractValidator::new()
    .with_strict_mode(true);

let result = validator.validate_openapi(
    &spec,
    "https://api.production.com"
).await;

if !result.passed {
    println!("Validation failed!");
    for error in &result.errors {
        println!("  - {}: {}", error.path, error.message);
    }
}
```

### Breaking Changes Detection

Compare two API versions:

```rust
let old_spec = OpenApiSpec::from_file("old-api.yaml").await?;
let new_spec = OpenApiSpec::from_file("new-api.yaml").await?;

let validator = ContractValidator::new();
let result = validator.compare_specs(&old_spec, &new_spec);

for change in &result.breaking_changes {
    println!("{:?}: {} - {}",
        change.severity,
        change.path,
        change.description
    );
}
```

### GitHub Actions Workflows

We provide three ready-to-use workflows:

#### 1. Contract Validation
`.github/workflows/contract-validation.yml`

- Validates mocks against live API
- Runs on PR and pushes to main
- Posts results as PR comment
- Fails build if validation fails

#### 2. Breaking Changes Detection
`.github/workflows/breaking-changes.yml`

- Compares PR changes against main branch
- Detects removed endpoints, required fields
- Warns on breaking changes
- Optional: block merge if breaking changes found

#### 3. Integration Tests
`.github/workflows/integration-tests.yml`

- Starts full mock service stack
- Runs integration test suite
- Collects coverage reports
- Archives test artifacts

### GitLab CI Pipeline

`.gitlab-ci.yml` includes:

- Build stage: Create Docker image
- Validate stage: Contract validation & breaking change detection
- Test stage: Integration tests with mock services
- Deploy stage: Staging and production deployment

Stages with validation, testing, and deployment to staging/production.

### Jenkins Pipeline

`Jenkinsfile` provides:

- Multi-stage pipeline with proper error handling
- Contract validation on PRs
- Breaking change detection
- Integration testing with mocks
- Deployment to staging/production
- Artifact archiving and reporting

### CI/CD Best Practices

1. **Always validate contracts** on PR before merging
2. **Block merges** if critical breaking changes detected
3. **Run integration tests** against mocks before deploying
4. **Archive validation reports** for debugging
5. **Use health checks** to ensure services are ready
6. **Set timeouts** for service startup (60s recommended)
7. **Clean up resources** in `always` blocks

---

## 🚀 Complete Workflow Example

Here's how all pieces work together:

### 1. Development Phase

```bash
# Developer works on API changes
code specs/api.yaml

# VS Code extension shows live mocks
# Developer tests changes locally
```

### 2. Pull Request

```yaml
# GitHub Actions runs automatically:
1. Contract validation against staging API
2. Breaking changes detection vs main branch
3. Integration tests with docker-compose
```

### 3. Code Review

```markdown
## Contract Validation Results ✅

**Status**: PASSED
**Total Checks**: 15
**Passed**: 15
**Failed**: 0

## Breaking Changes ⚠️

- **RequiredFieldAdded** (Major): /api/users - Added required field `email`
- **EndpointRemoved** (Critical): /api/legacy/endpoint removed
```

### 4. Deployment

```bash
# Merge triggers deployment pipeline:
1. Build Docker images
2. Deploy to staging
3. Run smoke tests
4. Manual approval for production
5. Deploy to production
```

---

## 📊 Monitoring & Observability

### Management API Endpoints

All workflow tools use these endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/__mockforge/api/health` | GET | Health check |
| `/__mockforge/api/stats` | GET | Server statistics |
| `/__mockforge/api/config` | GET | Server configuration |
| `/__mockforge/api/mocks` | GET | List all mocks |
| `/__mockforge/api/mocks` | POST | Create mock |
| `/__mockforge/api/mocks/:id` | PUT | Update mock |
| `/__mockforge/api/mocks/:id` | DELETE | Delete mock |
| `/__mockforge/api/export` | GET | Export mocks (JSON/YAML) |
| `/__mockforge/api/import` | POST | Import mocks |
| `/__mockforge/ws` | WebSocket | Live updates |

### WebSocket Events

Real-time notifications:

```json
{
  "type": "mock_created",
  "mock": { ... },
  "timestamp": "2024-01-15T10:30:00Z"
}

{
  "type": "mock_updated",
  "mock": { ... },
  "timestamp": "2024-01-15T10:31:00Z"
}

{
  "type": "stats_updated",
  "stats": {
    "uptime_seconds": 3600,
    "total_requests": 1250,
    "active_mocks": 15
  },
  "timestamp": "2024-01-15T10:32:00Z"
}
```

---

## 🔧 Troubleshooting

### VS Code Extension

**Issue**: Extension not connecting to server

```bash
# Check server is running
curl http://localhost:3000/__mockforge/api/health

# Check WebSocket is available
wscat -c ws://localhost:3000/__mockforge/ws
```

**Issue**: Mocks not updating in tree view

- Check WebSocket connection in VS Code Output panel
- Verify `mockforge.autoConnect` is true
- Reload VS Code window

### Docker Compose

**Issue**: Services not starting

```bash
# Check logs
docker-compose logs

# Rebuild images
docker-compose build --no-cache

# Check port conflicts
netstat -tlnp | grep -E '3001|3002|3003|3004'
```

### CI/CD Pipelines

**Issue**: Contract validation failing

- Ensure API URL is accessible from CI
- Check authentication/API keys
- Verify OpenAPI spec syntax
- Review validation reports in artifacts

---

## 📚 Additional Resources

- [VS Code Extension API Docs](https://code.visualstudio.com/api)
- [Docker Compose Guide](./DOCKER_COMPOSE_GUIDE.md)
- [OpenAPI Specification](https://swagger.io/specification/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [GitLab CI/CD Docs](https://docs.gitlab.com/ee/ci/)
- [Jenkins Pipeline](https://www.jenkins.io/doc/book/pipeline/)

---

## ✨ Summary

MockForge now provides **complete developer workflow integration**:

✅ **VS Code Extension** - Visual mock management directly in your IDE
✅ **API Collection Sync** - Seamless Postman/Insomnia/Hoppscotch integration
✅ **Docker Compose Autogen** - One-command microservices testing environment
✅ **CI/CD Validation** - Automated contract verification and breaking change detection

These features position MockForge as **the backbone of your development pipeline**, not just a standalone mocking tool.

---

**Next Steps**:

1. Install the VS Code extension
2. Export your OpenAPI specs to Postman
3. Generate docker-compose for local testing
4. Add CI/CD validation to your pipeline
5. Enjoy seamless API development! 🚀
