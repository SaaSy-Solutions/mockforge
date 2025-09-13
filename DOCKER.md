# Docker Setup for MockForge

This guide covers how to run MockForge using Docker and Docker Compose for both development and production use cases.

## 🚀 Quick Start

### Using Docker Compose (Recommended)

```bash
# Clone the repository
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge

# Start MockForge with Docker Compose
make docker-compose-up

# Or build and start
make docker-compose-build
```

MockForge will be available at:
- **HTTP API**: http://localhost:3000
- **WebSocket**: ws://localhost:3001
- **Admin UI**: http://localhost:8080
- **gRPC**: localhost:50051

### Using Docker Directly

```bash
# Build the image
make docker-build

# Run the container
make docker-run
```

## 📋 Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+ (for compose functionality)
- At least 2GB of available RAM
- Ports 3000, 3001, 50051, and 8080 available

## 🏗️ Docker Configuration

### Production Dockerfile

The main `Dockerfile` is optimized for production use:

- **Multi-stage build** for smaller final image size
- **Minimal runtime** using Debian slim base
- **Non-root user** for security
- **Only essential dependencies** included

### Development Setup

For development with hot reload, use the development configuration:

```bash
# Start development environment
make docker-dev

# View logs
make docker-compose-logs

# Stop development environment
make docker-compose-down
```

## 🔧 Configuration

### Environment Variables

Configure MockForge through environment variables:

```bash
# Basic configuration
export MOCKFORGE_HTTP_PORT=3000
export MOCKFORGE_WS_PORT=3001
export MOCKFORGE_ADMIN_PORT=8080
export MOCKFORGE_GRPC_PORT=50051

# Feature flags
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true
export MOCKFORGE_LATENCY_ENABLED=true

# File paths
export MOCKFORGE_HTTP_OPENAPI_SPEC=examples/openapi-demo.json
export MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl
```

### Volume Mounts

The Docker setup includes these volume mounts:

```yaml
volumes:
  - ./examples:/app/examples:ro    # Read-only examples
  - ./fixtures:/app/fixtures       # Read-write fixtures
  - ./logs:/app/logs               # Application logs
  - ./config.yaml:/app/config.yaml:ro  # Custom configuration
```

## 📖 Usage Examples

### Basic API Mocking

```bash
# Start with OpenAPI spec
docker run -p 3000:3000 \
  -e MOCKFORGE_HTTP_OPENAPI_SPEC=examples/openapi-demo.json \
  -e MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
  mockforge

# Test the API
curl http://localhost:3000/ping
```

### WebSocket Mocking

```bash
# Start with WebSocket replay
docker run -p 3001:3001 \
  -e MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
  mockforge
```

### Full Stack with Admin UI

```bash
# Start all services
docker run -p 3000:3000 -p 3001:3001 -p 8080:8080 \
  -e MOCKFORGE_ADMIN_ENABLED=true \
  -e MOCKFORGE_HTTP_OPENAPI_SPEC=examples/openapi-demo.json \
  -e MOCKFORGE_WS_REPLAY_FILE=examples/ws-demo.jsonl \
  mockforge

# Access Admin UI at http://localhost:8080
```

### Using Docker Compose

Create a `docker-compose.override.yml` for custom configuration:

```yaml
version: '3.8'

services:
  mockforge:
    environment:
      - MOCKFORGE_LOG_LEVEL=debug
      - MOCKFORGE_LATENCY_ENABLED=false
      - MOCKFORGE_HTTP_OPENAPI_SPEC=examples/custom-api.json
    volumes:
      - ./my-examples:/app/examples:ro
      - ./my-config.yaml:/app/config.yaml:ro
```

## 🔍 Debugging and Troubleshooting

### View Logs

```bash
# View container logs
make docker-compose-logs

# Or directly with Docker
docker logs mockforge

# Follow logs in real-time
docker logs -f mockforge
```

### Access Container Shell

```bash
# Get a shell in the running container
docker exec -it mockforge /bin/bash

# Check running processes
docker exec mockforge ps aux

# View environment variables
docker exec mockforge env | grep MOCKFORGE
```

### Common Issues

#### Port Already in Use

```bash
# Check what's using the ports
netstat -tlnp | grep :3000

# Use different ports
docker run -p 3001:3000 -p 3002:3001 mockforge
```

#### Permission Issues

```bash
# Fix volume permissions
sudo chown -R 1000:1000 fixtures/
sudo chown -R 1000:1000 logs/
```

#### Build Issues

```bash
# Clear Docker cache
docker system prune -a

# Rebuild without cache
docker build --no-cache -t mockforge .
```

## 🏭 Production Deployment

### Using Docker Compose in Production

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  mockforge:
    image: mockforge:latest
    ports:
      - "80:3000"    # HTTP
      - "8081:8080"  # Admin UI (internal only)
    environment:
      - MOCKFORGE_LOG_LEVEL=warn
      - MOCKFORGE_ADMIN_ENABLED=true
    restart: always
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### Health Checks

The container includes built-in health checks:

```bash
# Check container health
docker ps

# View health status
docker inspect mockforge | grep -A 10 "Health"
```

### Resource Limits

Set resource limits for production:

```yaml
services:
  mockforge:
    deploy:
      resources:
        limits:
          cpus: '1.0'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 256M
```

## 🔐 Security Considerations

### Production Security

- **Run as non-root user**: The container runs as a non-privileged user
- **Minimal attack surface**: Only essential packages installed
- **No development tools**: Production image excludes development dependencies

### Environment Variables

- **Don't hardcode secrets**: Use Docker secrets or external configuration
- **Use environment files**: Store sensitive config in `.env` files (excluded from git)

```bash
# .env file (don't commit to git)
MOCKFORGE_ADMIN_PASSWORD=secure-password
DATABASE_URL=postgresql://user:pass@db:5432/mockforge
```

## 📊 Monitoring

### Logging

Logs are available through Docker:

```bash
# View recent logs
docker logs --tail 100 mockforge

# Follow logs
docker logs -f mockforge

# Save logs to file
docker logs mockforge > mockforge.log 2>&1
```

### Metrics

The Admin UI provides metrics at runtime. For production monitoring:

```bash
# Health check endpoint
curl http://localhost:3000/health

# Admin metrics (if enabled)
curl http://localhost:8080/__mockforge/metrics
```

## 🚀 CI/CD Integration

### GitHub Actions Example

```yaml
# .github/workflows/docker.yml
name: Build and Push Docker Image

on:
  push:
    branches: [ main ]

jobs:
  docker:
    runs-on: ubuntu-latest

    steps:
    - name: Checkout code
      uses: actions/checkout@v3

    - name: Set up Docker Buildx
      uses: actions/docker/setup-buildx-action@v2

    - name: Build and push
      uses: actions/docker/build-push-action@v4
      with:
        context: .
        push: true
        tags: saasy-solutions/mockforge:latest
```

## 📚 Additional Resources

- [MockForge User Guide](../book/src/README.md)
- [Configuration Reference](../book/src/configuration/)
- [Environment Variables Guide](../book/src/configuration/environment.md)
- [Docker Compose Documentation](https://docs.docker.com/compose/)

## 🆘 Support

If you encounter issues:

1. Check the [troubleshooting guide](../book/src/reference/troubleshooting.md)
2. Review Docker logs: `docker logs mockforge`
3. Verify port availability: `netstat -tlnp | grep :3000`
4. Test basic connectivity: `docker exec mockforge curl http://localhost:3000/ping`

For additional help, see the main [README](../README.md) or create an issue on GitHub.
