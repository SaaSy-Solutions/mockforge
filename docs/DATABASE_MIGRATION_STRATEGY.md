# Database Migration Strategy for Production

This document outlines the database migration strategy for MockForge Cloud production deployments.

## Overview

MockForge uses SQLx migrations for database schema management. Migrations are automatically run on application startup, but for production deployments, we recommend a more controlled approach.

## Migration System

### Technology Stack

- **Migration Tool**: SQLx (`sqlx migrate`)
- **Database**: PostgreSQL (Neon, RDS, or self-hosted)
- **Migration Location**: `crates/mockforge-registry-server/migrations/`
- **Migration Format**: SQL files with timestamped naming

### Current Implementation

Migrations are automatically executed on application startup:

```rust
// In src/main.rs
db.migrate().await?;
```

This uses the `sqlx::migrate!()` macro which:
- Scans the `./migrations/` directory
- Executes migrations in order
- Tracks applied migrations in the `_sqlx_migrations` table
- Skips already-applied migrations

## Production Migration Strategy

### Option 1: Automatic Migrations (Current - Recommended for Small Deployments)

**When to Use**:
- Small deployments with minimal downtime tolerance
- Single-instance deployments
- Development/staging environments

**How It Works**:
- Migrations run automatically on application startup
- Application waits for migrations to complete before accepting traffic
- If migrations fail, the application doesn't start

**Pros**:
- Simple, no manual intervention
- Ensures schema is always up-to-date
- No separate migration step required

**Cons**:
- All instances run migrations (can cause conflicts)
- No pre-migration validation
- Downtime during migration execution

**Best Practices**:
1. Always test migrations in staging first
2. Use zero-downtime migrations when possible
3. Deploy with a single instance first, then scale
4. Monitor migration execution time

### Option 2: Manual Migrations (Recommended for Production)

**When to Use**:
- Multi-instance deployments
- Large databases with long-running migrations
- Need for pre-migration validation
- Compliance requirements

**How It Works**:
1. Disable automatic migrations in application code
2. Run migrations manually before deployment
3. Deploy application code after migrations complete

**Implementation**:

1. **Disable automatic migrations** (add environment variable check):

```rust
// In src/main.rs
if std::env::var("AUTO_MIGRATE").unwrap_or_else(|_| "false".to_string()) == "true" {
    db.migrate().await?;
    tracing::info!("Database migrations complete");
} else {
    tracing::info!("Automatic migrations disabled (set AUTO_MIGRATE=true to enable)");
}
```

2. **Run migrations manually**:

```bash
# Using sqlx-cli
sqlx migrate run --database-url $DATABASE_URL

# Or using a migration script
./scripts/migrate.sh
```

3. **Deploy application** after migrations succeed

**Pros**:
- Full control over migration timing
- Can validate migrations before deployment
- Prevents multiple instances from running migrations
- Can run migrations during maintenance windows

**Cons**:
- Requires manual step in deployment process
- Risk of deploying code before migrations
- More complex deployment pipeline

### Option 3: Migration Service (Recommended for Large Scale)

**When to Use**:
- Very large deployments
- Multiple environments
- Need for migration orchestration
- CI/CD integration

**How It Works**:
- Separate migration service/job runs migrations
- Application checks migration status on startup
- Application fails to start if migrations are pending

**Implementation**:

1. **Create migration job** (Kubernetes Job, Fly.io job, etc.):

```yaml
# kubernetes/migration-job.yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: database-migration
spec:
  template:
    spec:
      containers:
      - name: migrate
        image: mockforge-registry:latest
        command: ["sqlx", "migrate", "run"]
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: database-secret
              key: url
      restartPolicy: Never
```

2. **Application checks migration status**:

```rust
// In src/main.rs
async fn check_migrations(pool: &PgPool) -> Result<()> {
    let pending = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM _sqlx_migrations WHERE applied_at IS NULL"
    )
    .fetch_one(pool)
    .await?;

    if pending > 0 {
        anyhow::bail!("Pending migrations detected. Please run migrations before starting the application.");
    }

    Ok(())
}
```

## Migration Best Practices

### 1. Zero-Downtime Migrations

**Additive Changes** (No Downtime):
- Adding new columns with defaults
- Adding new tables
- Adding indexes (concurrently)
- Adding new functions/views

**Example**:
```sql
-- Add new column with default (no downtime)
ALTER TABLE users ADD COLUMN email_verified BOOLEAN DEFAULT false;

-- Add index concurrently (no downtime)
CREATE INDEX CONCURRENTLY idx_users_email_verified ON users(email_verified);
```

**Breaking Changes** (Require Downtime):
- Dropping columns
- Changing column types
- Dropping tables
- Adding NOT NULL constraints without defaults

**Example**:
```sql
-- Multi-step process for adding NOT NULL constraint
-- Step 1: Add column with default (no downtime)
ALTER TABLE users ADD COLUMN new_field TEXT;

-- Step 2: Backfill data (no downtime, but may take time)
UPDATE users SET new_field = 'default_value' WHERE new_field IS NULL;

-- Step 3: Add NOT NULL constraint (requires brief lock)
ALTER TABLE users ALTER COLUMN new_field SET NOT NULL;
```

### 2. Migration Naming Convention

Use descriptive, timestamped names:

```
YYYYMMDDHHMMSS_description.sql
```

Examples:
- `20250127120000_add_organizations_table.sql`
- `20250127120001_add_org_members_table.sql`
- `20250127120002_add_subscriptions_table.sql`

### 3. Migration Testing

**Before Production**:
1. Test migrations on a copy of production data
2. Measure migration execution time
3. Verify rollback procedures
4. Test application compatibility

**Testing Script**:
```bash
#!/bin/bash
# scripts/test-migration.sh

set -e

echo "Creating test database..."
createdb mockforge_test

echo "Running migrations..."
sqlx migrate run --database-url postgresql://localhost/mockforge_test

echo "Verifying schema..."
psql mockforge_test -c "\d"

echo "Testing rollback..."
sqlx migrate revert --database-url postgresql://localhost/mockforge_test

echo "Cleaning up..."
dropdb mockforge_test
```

### 4. Rollback Strategy

**SQLx Rollback**:
```bash
# Revert last migration
sqlx migrate revert --database-url $DATABASE_URL

# Revert multiple migrations
sqlx migrate revert --database-url $DATABASE_URL --repeat 3
```

**Manual Rollback**:
Create a separate rollback migration file:

```sql
-- migrations/20250127120003_rollback_add_feature.sql
-- Rollback for 20250127120002_add_feature.sql

DROP TABLE IF EXISTS feature_table;
```

**Important**: Always test rollbacks on staging before production!

### 5. Migration Validation

**Pre-Migration Checks**:
- Database backup exists
- Sufficient disk space
- Migration file syntax is valid
- No conflicting migrations

**Post-Migration Checks**:
- All migrations applied successfully
- Schema matches expected state
- Application starts successfully
- No data corruption

## Deployment Procedures

### Standard Deployment (Automatic Migrations)

```bash
# 1. Backup database
pg_dump $DATABASE_URL > backup_$(date +%Y%m%d_%H%M%S).sql

# 2. Deploy application (migrations run automatically)
fly deploy

# 3. Verify deployment
curl https://api.mockforge.dev/health
```

### Controlled Deployment (Manual Migrations)

```bash
# 1. Backup database
pg_dump $DATABASE_URL > backup_$(date +%Y%m%d_%H%M%S).sql

# 2. Run migrations
sqlx migrate run --database-url $DATABASE_URL

# 3. Verify migrations
sqlx migrate info --database-url $DATABASE_URL

# 4. Deploy application (with AUTO_MIGRATE=false)
fly deploy --env AUTO_MIGRATE=false

# 5. Verify deployment
curl https://api.mockforge.dev/health
```

### Zero-Downtime Deployment

```bash
# 1. Backup database
pg_dump $DATABASE_URL > backup_$(date +%Y%m%d_%H%M%S).sql

# 2. Run additive migrations (if any)
sqlx migrate run --database-url $DATABASE_URL

# 3. Deploy new application version (blue-green or rolling)
fly deploy --strategy rolling

# 4. Run breaking migrations (if any) during maintenance window
# 5. Verify deployment
```

## Monitoring and Alerts

### Migration Status Monitoring

Add health check endpoint that verifies migration status:

```rust
// In handlers/health.rs
pub async fn migration_status(State(state): State<AppState>) -> Json<Value> {
    let pool = state.db.pool();

    let pending = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM _sqlx_migrations WHERE applied_at IS NULL"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Json(json!({
        "migrations_pending": pending,
        "status": if pending > 0 { "warning" } else { "ok" }
    }))
}
```

### Alerting

Set up alerts for:
- Pending migrations detected
- Migration failures
- Long-running migrations (> 5 minutes)
- Schema drift detected

## Emergency Procedures

### Migration Failure

1. **Stop application deployment** immediately
2. **Assess impact**:
   - Check migration logs
   - Verify database state
   - Check for partial migrations
3. **Rollback if necessary**:
   ```bash
   sqlx migrate revert --database-url $DATABASE_URL
   ```
4. **Restore from backup** if rollback fails:
   ```bash
   psql $DATABASE_URL < backup_YYYYMMDD_HHMMSS.sql
   ```
5. **Fix migration** and test before retrying

### Schema Drift

If schema doesn't match expected state:

1. **Document current state**:
   ```bash
   pg_dump --schema-only $DATABASE_URL > current_schema.sql
   ```
2. **Compare with expected**:
   ```bash
   diff current_schema.sql expected_schema.sql
   ```
3. **Create corrective migration** if needed
4. **Apply corrective migration**

## Migration Checklist

### Before Creating Migration

- [ ] Migration is necessary (can't be handled in application code?)
- [ ] Migration is backward compatible (or has rollback plan)
- [ ] Migration tested on staging database
- [ ] Migration execution time estimated
- [ ] Rollback procedure documented

### Before Deploying Migration

- [ ] Database backup created
- [ ] Migration tested on staging
- [ ] Rollback tested on staging
- [ ] Deployment plan documented
- [ ] Team notified of deployment
- [ ] Monitoring/alerting configured

### After Deploying Migration

- [ ] Migration applied successfully
- [ ] Application starts successfully
- [ ] Health checks passing
- [ ] No errors in logs
- [ ] Performance metrics normal
- [ ] Rollback plan ready if issues occur

## Tools and Scripts

### Migration Helper Script

Create `scripts/migrate.sh`:

```bash
#!/bin/bash
set -e

DATABASE_URL=${DATABASE_URL:-"postgresql://localhost/mockforge"}

echo "Running database migrations..."
sqlx migrate run --database-url "$DATABASE_URL"

echo "Migration status:"
sqlx migrate info --database-url "$DATABASE_URL"
```

### Migration Status Check

Create `scripts/check-migrations.sh`:

```bash
#!/bin/bash
set -e

DATABASE_URL=${DATABASE_URL:-"postgresql://localhost/mockforge"}

echo "Checking migration status..."
sqlx migrate info --database-url "$DATABASE_URL"

PENDING=$(sqlx query "SELECT COUNT(*) FROM _sqlx_migrations WHERE applied_at IS NULL" --database-url "$DATABASE_URL" | grep -o '[0-9]\+')

if [ "$PENDING" -gt 0 ]; then
    echo "WARNING: $PENDING pending migrations detected!"
    exit 1
else
    echo "All migrations applied."
    exit 0
fi
```

## References

- [SQLx Migrations Documentation](https://github.com/launchbadge/sqlx/blob/main/sqlx-macros/README.md#migrate)
- [PostgreSQL Migration Best Practices](https://www.postgresql.org/docs/current/ddl-alter.html)
- [Zero-Downtime Migrations Guide](https://www.braintreepayments.com/blog/safe-operations-for-high-volume-postgresql/)

## Support

For migration issues or questions:
- Check migration logs: `fly logs --app mockforge-registry`
- Review migration history: `sqlx migrate info --database-url $DATABASE_URL`
- Contact: devops@mockforge.dev
