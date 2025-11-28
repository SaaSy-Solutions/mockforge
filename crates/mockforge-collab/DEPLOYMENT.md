# MockForge Collaboration - Deployment Guide

## Prerequisites

- Rust 1.75 or later
- SQLite 3.35+ or PostgreSQL 13+
- Docker (optional, for containerized deployment)

## Building

### Local Development

```bash
# Set up database URL for sqlx compile-time checking
export DATABASE_URL="sqlite://mockforge-collab.db"

# Create the database
sqlx database create

# Run migrations
sqlx migrate run --source crates/mockforge-collab/migrations

# Build the crate
cargo build --package mockforge-collab

# For offline mode (no database needed during compilation)
export SQLX_OFFLINE=true
cargo sqlx prepare --package mockforge-collab
cargo build --package mockforge-collab
```

### Production Build

```bash
cargo build --release --package mockforge-collab
```

## Configuration

### Environment Variables

```bash
# Required
MOCKFORGE_JWT_SECRET=your-secure-secret-key-here
MOCKFORGE_DATABASE_URL=sqlite://mockforge-collab.db

# Optional
MOCKFORGE_BIND_ADDRESS=0.0.0.0:8080
MOCKFORGE_MAX_CONNECTIONS=100
MOCKFORGE_AUTO_COMMIT=true
```

### Configuration File

Create `mockforge-collab.toml`:

```toml
[server]
bind_address = "0.0.0.0:8080"
jwt_secret = "your-secure-secret"

[database]
url = "sqlite://mockforge-collab.db"
# Or for PostgreSQL:
# url = "postgresql://user:password@localhost/mockforge"

[collaboration]
max_connections_per_workspace = 100
event_bus_capacity = 1000
auto_commit = true
session_timeout_hours = 24
websocket_ping_interval_secs = 30
max_message_size_bytes = 1048576
```

## Database Setup

### SQLite (Self-Hosted)

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features sqlite

# Create database
sqlx database create --database-url sqlite://mockforge-collab.db

# Run migrations
sqlx migrate run \
  --source crates/mockforge-collab/migrations \
  --database-url sqlite://mockforge-collab.db
```

### PostgreSQL (Cloud/Production)

```bash
# Install sqlx-cli with postgres support
cargo install sqlx-cli --features postgres

# Create database
sqlx database create --database-url postgresql://user:pass@localhost/mockforge

# Run migrations
sqlx migrate run \
  --source crates/mockforge-collab/migrations \
  --database-url postgresql://user:pass@localhost/mockforge
```

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# Build with offline mode
ENV SQLX_OFFLINE=true
RUN cargo build --release --package mockforge-collab

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mockforge-collab /usr/local/bin/
COPY --from=builder /app/crates/mockforge-collab/migrations /migrations

# Create data directory
RUN mkdir -p /data

ENV MOCKFORGE_DATABASE_URL=sqlite:///data/mockforge-collab.db
ENV MOCKFORGE_BIND_ADDRESS=0.0.0.0:8080

EXPOSE 8080

CMD ["mockforge-collab"]
```

### Docker Compose

```yaml
version: '3.8'

services:
  mockforge-collab:
    build: .
    ports:
      - "8080:8080"
    environment:
      - MOCKFORGE_JWT_SECRET=${JWT_SECRET}
      - MOCKFORGE_DATABASE_URL=postgresql://mockforge:password@postgres:5432/mockforge
    depends_on:
      - postgres
    volumes:
      - ./data:/data

  postgres:
    image: postgres:15-alpine
    environment:
      - POSTGRES_USER=mockforge
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=mockforge
    volumes:
      - postgres-data:/var/lib/postgresql/data

volumes:
  postgres-data:
```

## Kubernetes Deployment

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mockforge-collab
  labels:
    app: mockforge-collab
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mockforge-collab
  template:
    metadata:
      labels:
        app: mockforge-collab
    spec:
      containers:
      - name: mockforge-collab
        image: your-registry/mockforge-collab:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 8080
          name: websocket
          protocol: TCP
        env:
        - name: MOCKFORGE_JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: mockforge-secrets
              key: jwt-secret
        - name: MOCKFORGE_DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: mockforge-secrets
              key: database-url
        - name: MOCKFORGE_BIND_ADDRESS
          value: "0.0.0.0:8080"
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: mockforge-collab
spec:
  type: LoadBalancer
  selector:
    app: mockforge-collab
  ports:
  - port: 80
    targetPort: 8080
    name: http
```

### Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: mockforge-secrets
type: Opaque
stringData:
  jwt-secret: "your-secure-secret-key"
  database-url: "postgresql://user:pass@postgres-service:5432/mockforge"
```

## Security Considerations

1. **JWT Secret**: Use a strong, randomly generated secret (at least 32 characters)
   ```bash
   openssl rand -base64 32
   ```

2. **Database Credentials**: Never commit database passwords to version control

3. **TLS/SSL**: Always use HTTPS in production
   - Configure a reverse proxy (nginx, Traefik) with Let's Encrypt
   - Or use cloud provider load balancers with SSL termination

4. **Network Security**:
   - Restrict database access to the application network
   - Use firewalls to limit inbound connections
   - Enable VPC/private networking in cloud environments

5. **Rate Limiting**: Implement rate limiting at the reverse proxy level

## Monitoring

### Health Endpoints

- `GET /health` - Health check (returns 200 if healthy)
- `GET /ready` - Readiness check (returns 200 when ready to accept traffic)
- `GET /metrics` - Prometheus metrics

### Logging

Configure log level with environment variable:

```bash
RUST_LOG=mockforge_collab=info,sqlx=warn
```

### Metrics

The server exports Prometheus metrics at `/metrics`:

- `mockforge_collab_active_connections` - Number of active WebSocket connections
- `mockforge_collab_workspaces_total` - Total number of workspaces
- `mockforge_collab_commits_total` - Total number of commits
- `mockforge_collab_events_published` - Number of events published

## Backup and Recovery

### SQLite Backup

```bash
# Backup
sqlite3 mockforge-collab.db ".backup backup-$(date +%Y%m%d).db"

# Restore
cp backup-20240101.db mockforge-collab.db
```

### PostgreSQL Backup

```bash
# Backup
pg_dump -U mockforge mockforge > backup-$(date +%Y%m%d).sql

# Restore
psql -U mockforge mockforge < backup-20240101.sql
```

## Scaling

### Horizontal Scaling

The collaboration server can be scaled horizontally with a shared database and Redis for session state:

1. Deploy multiple instances behind a load balancer
2. Use PostgreSQL for the shared database
3. Add Redis for WebSocket session affinity (future enhancement)

### Vertical Scaling

- Increase memory for large workspaces with many members
- Increase CPU for high concurrent connection counts

## Troubleshooting

### Common Issues

1. **Database Connection Errors**
   - Check DATABASE_URL is correctly set
   - Verify database is accessible
   - Check migrations have been run

2. **WebSocket Connection Failures**
   - Ensure WebSocket protocol is allowed in reverse proxy
   - Check firewall rules
   - Verify JWT token is valid

3. **High Memory Usage**
   - Reduce max_connections_per_workspace
   - Reduce event_bus_capacity
   - Check for memory leaks with monitoring tools

### Logs

Check logs for errors:

```bash
# Docker
docker logs mockforge-collab

# Kubernetes
kubectl logs -f deployment/mockforge-collab

# Systemd
journalctl -u mockforge-collab -f
```

## Support

For issues and questions:
- GitHub: https://github.com/SaaSy-Solutions/mockforge/issues
- Documentation: https://docs.mockforge.dev
