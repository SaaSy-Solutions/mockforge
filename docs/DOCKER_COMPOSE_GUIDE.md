# Docker Compose Guide for MockForge

## Overview

MockForge provides powerful Docker Compose integration for local microservices testing. This guide covers:

- Auto-generating docker-compose configurations
- Setting up networked mock services
- Best practices for integration testing

## Quick Start

### Single Service

```yaml
version: '3.8'

services:
  mockforge-api:
    image: mockforge:latest
    ports:
      - "3000:3000"
    environment:
      - MOCKFORGE_PORT=3000
      - MOCKFORGE_OPENAPI_SPEC=/specs/api.yaml
    volumes:
      - ./specs:/specs:ro
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 10s
      timeout: 5s
      retries: 3
```

### Multiple Microservices

Use the included `docker-compose.microservices.yml` for a complete microservices setup:

```bash
docker-compose -f docker-compose.microservices.yml up
```

This starts 4 interconnected mock services:
- **mock-auth** (port 3001) - Authentication service
- **mock-users** (port 3002) - User management (depends on auth)
- **mock-orders** (port 3003) - Order processing (depends on auth & users)
- **mock-payments** (port 3004) - Payment handling (depends on auth)

## Programmatic Generation

MockForge includes a Rust API for generating docker-compose files:

```rust
use mockforge_core::docker_compose::{DockerComposeGenerator, MockServiceSpec};

let generator = DockerComposeGenerator::new("my-network".to_string());

let services = vec![
    MockServiceSpec {
        name: "api".to_string(),
        port: 3000,
        spec_path: Some("api.yaml".to_string()),
        config_path: None,
    },
    MockServiceSpec {
        name: "auth".to_string(),
        port: 3001,
        spec_path: Some("auth.yaml".to_string()),
        config_path: None,
    },
];

let config = generator.generate(services);
let yaml = generator.to_yaml(&config)?;

std::fs::write("docker-compose.yml", yaml)?;
```

### With Service Dependencies

```rust
use std::collections::HashMap;

let mut dependencies = HashMap::new();
dependencies.insert("api".to_string(), vec!["auth".to_string()]);

let config = generator.generate_with_dependencies(services, dependencies);
```

## Directory Structure

Organize your project for docker-compose:

```
project/
├── docker-compose.yml
├── specs/
│   ├── auth.yaml
│   ├── users.yaml
│   ├── orders.yaml
│   └── payments.yaml
├── configs/
│   ├── auth-config.yaml
│   └── api-config.yaml
└── logs/
    └── (generated logs)
```

## Environment Variables

Configure MockForge services via environment:

| Variable | Description | Default |
|----------|-------------|---------|
| `MOCKFORGE_PORT` | HTTP server port | 3000 |
| `MOCKFORGE_OPENAPI_SPEC` | OpenAPI spec file path | - |
| `MOCKFORGE_CONFIG` | Config file path | - |
| `RUST_LOG` | Log level | info |
| `MOCKFORGE_LATENCY_ENABLED` | Enable latency simulation | true |
| `MOCKFORGE_FAILURES_ENABLED` | Enable failure injection | false |

## Networking

All services are connected via a bridge network (`mockforge-network`). Services can communicate using container names:

```bash
# From mock-api container
curl http://mock-auth:3001/health

# From mock-orders container
curl http://mock-users:3002/api/users
```

## Health Checks

All services include health check endpoints:

```bash
# Check service health
curl http://localhost:3001/health

# Response
{
  "status": "healthy",
  "service": "mockforge-http",
  "uptime": 120
}
```

Docker Compose waits for health checks before starting dependent services.

## Advanced Configurations

### Custom Network Configuration

```yaml
networks:
  mockforge-network:
    driver: bridge
    ipam:
      driver: default
      config:
        - subnet: 172.28.0.0/16
```

### Volume Mounts for Hot-Reload

```yaml
volumes:
  - ./specs:/specs:ro          # Read-only specs
  - ./configs:/configs:ro       # Read-only configs
  - ./logs:/logs                # Writable logs
  - ./fixtures:/fixtures:ro     # Test fixtures
```

### Resource Limits

```yaml
services:
  mock-api:
    # ... other config
    deploy:
      resources:
        limits:
          cpus: '0.5'
          memory: 512M
        reservations:
          cpus: '0.25'
          memory: 256M
```

## Integration Testing

### Using with Pytest

```python
import pytest
import requests
import subprocess

@pytest.fixture(scope="session", autouse=True)
def docker_compose():
    # Start services
    subprocess.run(["docker-compose", "up", "-d"], check=True)

    # Wait for health checks
    for port in [3001, 3002, 3003, 3004]:
        wait_for_health(f"http://localhost:{port}/health")

    yield

    # Teardown
    subprocess.run(["docker-compose", "down"], check=True)

def test_auth_flow():
    # Test against mock services
    response = requests.post("http://localhost:3001/auth/login",
                           json={"username": "test", "password": "test"})
    assert response.status_code == 200
```

### Using with Newman (Postman CLI)

```bash
# Start services
docker-compose up -d

# Run Postman collection against mocks
newman run collection.json \
  --env-var baseUrl=http://localhost:3001

# Cleanup
docker-compose down
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Start MockForge services
        run: docker-compose up -d

      - name: Wait for services
        run: |
          for port in 3001 3002 3003 3004; do
            timeout 60 bash -c "until curl -f http://localhost:$port/health; do sleep 2; done"
          done

      - name: Run integration tests
        run: npm test

      - name: Cleanup
        if: always()
        run: docker-compose down
```

## Troubleshooting

### Services not starting

```bash
# Check logs
docker-compose logs mock-auth

# Check service status
docker-compose ps

# Rebuild images
docker-compose build --no-cache
```

### Network issues

```bash
# Inspect network
docker network inspect mockforge-network

# Test connectivity
docker-compose exec mock-api curl http://mock-auth:3001/health
```

### Port conflicts

If ports are already in use, modify the port mappings:

```yaml
ports:
  - "13001:3001"  # Map external port 13001 to internal 3001
```

## Best Practices

1. **Use health checks** - Always include health checks for service dependencies
2. **Version lock** - Pin Docker image versions in production
3. **Resource limits** - Set CPU and memory limits to prevent resource exhaustion
4. **Separate networks** - Use different networks for different test suites
5. **Log aggregation** - Mount logs directory for debugging
6. **Clean up** - Always run `docker-compose down` after tests

## Examples

See the repository for complete examples:
- `docker-compose.yml` - Basic single-service setup
- `docker-compose.microservices.yml` - Multi-service microservices setup
- `docker-compose.dev.yml` - Development setup with hot-reload

## API Reference

See `crates/mockforge-core/src/docker_compose.rs` for the programmatic API documentation.
