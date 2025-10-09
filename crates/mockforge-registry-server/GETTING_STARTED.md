# Getting Started with MockForge Registry Server

This guide will help you get the MockForge Plugin Registry server up and running in minutes.

## Prerequisites

- **Rust** 1.75+ ([install](https://rustup.rs/))
- **Docker** and **Docker Compose** ([install](https://docs.docker.com/get-docker/))
- **Make** (usually pre-installed on Linux/macOS)

## Quick Start (5 Minutes)

### 1. Start Infrastructure

```bash
cd crates/mockforge-registry-server

# Start PostgreSQL and MinIO
make dev
```

This starts:
- **PostgreSQL** on `localhost:5432` (database: `mockforge_registry`)
- **MinIO** (S3-compatible storage)
  - API: `http://localhost:9000`
  - Console: `http://localhost:9001` (credentials: `minioadmin`/`minioadmin`)

### 2. Run Database Migrations

```bash
# Install sqlx-cli (one-time)
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations and seed data
make seed
```

This creates:
- Database schema (tables, indexes, triggers)
- Test users:
  - **Admin**: `admin@mockforge.dev` / `admin123`
  - **Test User**: `test@example.com` / `test123`
- Sample plugins:
  - `auth-jwt` (JWT authentication, 1.2K downloads, 4.8★)
  - `template-crypto` (Crypto templates, 892 downloads, 4.5★)
  - `datasource-csv` (CSV connector, 2.3K downloads, 4.9★)

### 3. Start the Server

```bash
make run
```

The server starts on `http://localhost:8080`

### 4. Test the API

Open a new terminal:

```bash
# Health check
curl http://localhost:8080/health

# Search plugins
curl -X POST http://localhost:8080/api/v1/plugins/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "auth",
    "page": 0,
    "per_page": 20,
    "sort": "downloads"
  }' | jq

# Get plugin details
curl http://localhost:8080/api/v1/plugins/auth-jwt | jq
```

## Development Workflow

### Environment Variables

Copy `.env.example` to `.env` and customize:

```bash
cp .env.example .env
```

Key variables:
- `DATABASE_URL` - PostgreSQL connection string
- `JWT_SECRET` - Secret for signing JWTs (change in production!)
- `S3_ENDPOINT` - MinIO endpoint (`http://localhost:9000` for local dev)

### Available Make Commands

```bash
make help        # Show all commands
make dev         # Start infrastructure (DB + MinIO)
make migrate     # Run database migrations
make seed        # Run migrations + seed data
make run         # Run registry server
make test        # Run tests
make build       # Build release binary
make logs        # Show Docker logs
make stop        # Stop all services
make clean       # Stop services and remove volumes
make reset       # Clean + restart everything
```

### Manual Startup (Without Make)

```bash
# 1. Start infrastructure
docker-compose up -d db minio minio-init

# 2. Run migrations
sqlx migrate run --database-url "postgres://postgres:password@localhost:5432/mockforge_registry"

# 3. Set environment
export DATABASE_URL="postgres://postgres:password@localhost:5432/mockforge_registry"
export JWT_SECRET="dev-secret-change-me"
export S3_ENDPOINT="http://localhost:9000"
export AWS_ACCESS_KEY_ID="minioadmin"
export AWS_SECRET_ACCESS_KEY="minioadmin"

# 4. Run server
cargo run --package mockforge-registry-server
```

## API Testing Examples

### Register a New User

```bash
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "johndoe",
    "email": "john@example.com",
    "password": "securepassword123"
  }' | jq

# Save the token from response
export TOKEN="<your-jwt-token>"
```

### Login

```bash
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@mockforge.dev",
    "password": "admin123"
  }' | jq
```

### Search Plugins

```bash
# Search by query
curl -X POST http://localhost:8080/api/v1/plugins/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "authentication",
    "sort": "downloads",
    "page": 0,
    "per_page": 10
  }' | jq

# Search by category
curl -X POST http://localhost:8080/api/v1/plugins/search \
  -H "Content-Type: application/json" \
  -d '{
    "category": "auth",
    "sort": "rating",
    "page": 0,
    "per_page": 20
  }' | jq
```

### Get Plugin Info

```bash
# Get plugin details
curl http://localhost:8080/api/v1/plugins/auth-jwt | jq

# Get specific version
curl http://localhost:8080/api/v1/plugins/auth-jwt/versions/1.2.0 | jq
```

### Publish a Plugin (Authenticated)

```bash
# First, build your plugin WASM
cd /path/to/your/plugin
cargo build --target wasm32-wasi --release

# Base64 encode the WASM
WASM_DATA=$(base64 -w 0 target/wasm32-wasi/release/your_plugin.wasm)

# Calculate checksum
CHECKSUM=$(sha256sum target/wasm32-wasi/release/your_plugin.wasm | awk '{print $1}')

# Publish
curl -X POST http://localhost:8080/api/v1/plugins/publish \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d "{
    \"name\": \"my-awesome-plugin\",
    \"version\": \"1.0.0\",
    \"description\": \"An awesome plugin that does amazing things\",
    \"category\": \"auth\",
    \"license\": \"MIT\",
    \"repository\": \"https://github.com/me/my-plugin\",
    \"tags\": [\"auth\", \"jwt\"],
    \"checksum\": \"$CHECKSUM\",
    \"file_size\": $(stat -f%z target/wasm32-wasi/release/your_plugin.wasm),
    \"wasm_data\": \"$WASM_DATA\"
  }" | jq
```

### Yank a Version (Authenticated)

```bash
curl -X DELETE http://localhost:8080/api/v1/plugins/my-plugin/versions/1.0.0 \
  -H "Authorization: Bearer $TOKEN" | jq
```

## Accessing MinIO Console

1. Open `http://localhost:9001` in your browser
2. Login with `minioadmin` / `minioadmin`
3. Browse the `mockforge-plugins` bucket to see uploaded plugins

## Troubleshooting

### Port Already in Use

If ports 5432, 8080, 9000, or 9001 are already in use:

```bash
# Check what's using the port
lsof -i :5432
lsof -i :8080

# Kill the process or change ports in docker-compose.yml
```

### Database Connection Issues

```bash
# Check if PostgreSQL is running
docker-compose ps

# View PostgreSQL logs
docker-compose logs db

# Reset database
make clean dev seed
```

### Migration Errors

```bash
# Check migration status
sqlx migrate info --database-url "postgres://postgres:password@localhost:5432/mockforge_registry"

# Revert last migration
sqlx migrate revert --database-url "postgres://postgres:password@localhost:5432/mockforge_registry"

# Re-run migrations
make seed
```

### S3/MinIO Issues

```bash
# Check MinIO logs
docker-compose logs minio

# Recreate bucket
docker-compose up -d minio-init

# Test S3 connection
curl http://localhost:9000/minio/health/live
```

## Next Steps

1. **Integrate with MockForge CLI**
   - Point CLI to `http://localhost:8080` instead of production registry
   - Test search/install/publish workflows

2. **Deploy to Staging**
   - See `docs/PLUGIN_MARKETPLACE_IMPLEMENTATION.md` for deployment guide
   - Set up domain (e.g., `registry.mockforge.dev`)
   - Configure SSL/TLS

3. **Add More Features**
   - Review system implementation
   - Plugin verification/badges
   - Download statistics
   - User management UI

## Resources

- **Implementation Guide**: `docs/PLUGIN_MARKETPLACE_IMPLEMENTATION.md`
- **API Documentation**: See `src/routes.rs` for all endpoints
- **Database Schema**: `migrations/20250101000001_init.sql`
- **Example Data**: `migrations/20250101000002_seed_data.sql`

## Support

- **GitHub Issues**: https://github.com/SaaSy-Solutions/mockforge/issues
- **Discord**: (link when available)
- **Docs**: https://docs.mockforge.dev

## License

MIT OR Apache-2.0
