# MockForge Registry Server

Central plugin registry backend for MockForge.

## Overview

This crate implements the REST API server that powers the MockForge plugin marketplace. It handles:

- Plugin search and discovery
- Plugin publishing and versioning
- User authentication and authorization
- Reviews and ratings
- Download statistics
- Plugin storage (S3-compatible)

## Architecture

```
┌─────────────────┐
│   CLI Client    │  mockforge plugin registry search ...
└────────┬────────┘
         │
         │ HTTPS
         │
┌────────▼────────────────────────────────────────┐
│          Registry Server (this crate)           │
│                                                  │
│  ┌────────────┐  ┌──────────┐  ┌────────────┐ │
│  │  Axum API  │──│   Auth   │──│  Storage   │ │
│  │  Handlers  │  │   (JWT)  │  │    (S3)    │ │
│  └─────┬──────┘  └──────────┘  └────────────┘ │
│        │                                        │
│  ┌─────▼───────────────────────────────────┐  │
│  │         PostgreSQL Database             │  │
│  │  (plugins, versions, users, reviews)    │  │
│  └─────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.75+
- PostgreSQL 15+
- S3-compatible storage (AWS S3, MinIO, etc.)

### Development Setup

1. **Start dependencies:**

```bash
# Using Docker Compose (recommended)
cd crates/mockforge-registry-server
docker-compose up -d
```

This starts:
- PostgreSQL on port 5432
- MinIO (S3) on ports 9000 (API) and 9001 (console)

2. **Configure environment:**

```bash
# Create .env file
cat > .env <<EOF
DATABASE_URL=postgres://postgres:password@localhost:5432/mockforge_registry
JWT_SECRET=your-secret-key-change-me
S3_BUCKET=mockforge-plugins
S3_REGION=us-east-1
S3_ENDPOINT=http://localhost:9000
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin
PORT=8080
EOF
```

3. **Run migrations:**

```bash
cargo install sqlx-cli
sqlx migrate run
```

4. **Start server:**

```bash
cargo run --package mockforge-registry-server
```

The server will start on `http://localhost:8080`.

### Testing the API

```bash
# Health check
curl http://localhost:8080/health

# Register user
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "email": "test@example.com",
    "password": "securepassword"
  }'

# Search plugins
curl -X POST http://localhost:8080/api/v1/plugins/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "auth",
    "page": 0,
    "per_page": 20
  }'
```

## API Endpoints

### Public Endpoints

- `GET /health` - Health check
- `POST /api/v1/plugins/search` - Search plugins
- `GET /api/v1/plugins/:name` - Get plugin details
- `GET /api/v1/plugins/:name/versions/:version` - Get version details
- `GET /api/v1/plugins/:name/reviews` - Get plugin reviews
- `GET /api/v1/stats` - Get registry statistics

### Authentication Endpoints

- `POST /api/v1/auth/register` - Register new user
- `POST /api/v1/auth/login` - Login and get JWT token

### Authenticated Endpoints (requires JWT)

- `POST /api/v1/plugins/publish` - Publish new plugin version
- `DELETE /api/v1/plugins/:name/versions/:version/yank` - Yank version
- `POST /api/v1/plugins/:name/reviews` - Submit review

### Admin Endpoints (requires admin role)

- `POST /api/v1/admin/plugins/:name/verify` - Verify plugin (add badge)

## Database Schema

See `docs/PLUGIN_MARKETPLACE_IMPLEMENTATION.md` for complete schema.

Key tables:
- `plugins` - Plugin metadata
- `plugin_versions` - Version information
- `users` - User accounts
- `reviews` - Plugin reviews
- `tags` - Tag catalog
- `plugin_tags` - Plugin-tag associations

## Deployment

### Docker

```bash
# Build image
docker build -t mockforge-registry:latest .

# Run container
docker run -p 8080:8080 \
  -e DATABASE_URL=... \
  -e JWT_SECRET=... \
  mockforge-registry:latest
```

### Production Checklist

- [ ] Set strong JWT_SECRET
- [ ] Configure production database (RDS, etc.)
- [ ] Set up S3 bucket with proper permissions
- [ ] Enable HTTPS/TLS
- [ ] Configure CORS for your domain
- [ ] Set up monitoring and logging
- [ ] Configure backups
- [ ] Enable rate limiting
- [ ] Set up CDN for plugin downloads

## Development

### Adding New Endpoints

1. Add handler to `src/handlers/`
2. Register route in `src/routes.rs`
3. Add database queries if needed
4. Write tests

### Running Tests

```bash
cargo test --package mockforge-registry-server
```

### Database Migrations

```bash
# Create new migration
sqlx migrate add <name>

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

## Configuration

All configuration via environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | - | PostgreSQL connection string |
| `JWT_SECRET` | Yes | - | Secret for signing JWT tokens |
| `S3_BUCKET` | No | `mockforge-plugins` | S3 bucket name |
| `S3_REGION` | No | `us-east-1` | S3 region |
| `S3_ENDPOINT` | No | - | Custom S3 endpoint (MinIO, etc.) |
| `PORT` | No | `8080` | Server port |
| `MAX_PLUGIN_SIZE` | No | `52428800` | Max plugin size (50MB) |
| `RATE_LIMIT_PER_MINUTE` | No | `60` | API rate limit |

## Security

- JWT-based authentication
- Password hashing with bcrypt
- Checksum verification for uploads
- Rate limiting
- CORS configuration
- SQL injection protection (SQLx)

## License

MIT OR Apache-2.0
