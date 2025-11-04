# Migration Guide: MockForge 0.2.x to 1.0

This guide helps you migrate from MockForge 0.2.x to 1.0.

## Overview

MockForge 1.0 introduces several improvements and some breaking changes. This guide covers:
- Breaking changes and how to address them
- New features you should adopt
- Deprecated functionality
- Configuration changes

## Breaking Changes

### 1. Health Check Endpoints

**Before (0.2.x)**:
```bash
GET /health
```

**After (1.0)**:
```bash
GET /health          # Backwards compatible, but deprecated
GET /health/live     # Kubernetes liveness probe
GET /health/ready    # Kubernetes readiness probe
GET /health/startup  # Kubernetes startup probe
```

**Migration**:
- Update Kubernetes deployments to use `/health/ready` instead of `/health`
- Use `/health/live` for liveness probes
- No changes needed for basic health checks (backwards compatible)

**Example Kubernetes Configuration**:
```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 3000
readinessProbe:
  httpGet:
    path: /health/ready
    port: 3000
```

### 2. Error Handling

**Before (0.2.x)**:
Some error handling used `unwrap()` which could panic.

**After (1.0)**:
All critical error paths return proper `Result` types.

**Migration**:
- No code changes needed for API consumers
- Internal error handling improved (no breaking API changes)

### 3. Configuration Structure

**Before (0.2.x)**:
Some configuration options were scattered.

**After (1.0)**:
Consolidated configuration with better defaults.

**Migration**:
Review your configuration files. Most configurations are backwards compatible, but some defaults may have changed.

**Example Configuration Update**:
```yaml
# 0.2.x
server:
  port: 3000

# 1.0 (backwards compatible, but new options available)
server:
  port: 3000
  health:
    enabled: true
    init_timeout_seconds: 60
```

### 4. Plugin API (if using plugins)

**Before (0.2.x)**:
Some plugin APIs were experimental.

**After (1.0)**:
Plugin APIs are stable and fully documented.

**Migration**:
- Review plugin code for deprecated APIs
- Update to use stable APIs
- Check plugin documentation for changes

### 5. Observability

**Before (0.2.x)**:
Basic metrics and logging.

**After (1.0)**:
Enhanced observability with OpenTelemetry and business metrics.

**Migration**:
- No breaking changes for basic usage
- New metrics available (optional to adopt)
- Enhanced tracing support (optional)

## New Features to Adopt

### 1. Enhanced Health Checks

Take advantage of the new Kubernetes-native health checks:

```rust
// In your code (if using the library)
use mockforge_http::HealthManager;

let health_manager = HealthManager::with_init_timeout(Duration::from_secs(60));
// ... use health_manager
```

### 2. Production-Ready Tunnel Service

The tunnel service now includes:
- Persistent storage
- Rate limiting
- TLS support
- Audit logging

**Migration**:
```bash
# Enable persistent storage
TUNNEL_DATABASE_PATH=/var/lib/mockforge/tunnels.db tunnel-server

# Enable TLS
TUNNEL_TLS_CERT=/path/to/cert.pem \
TUNNEL_TLS_KEY=/path/to/key.pem \
tunnel-server

# Configure rate limiting
TUNNEL_RATE_LIMIT_ENABLED=true \
TUNNEL_RATE_LIMIT_RPM=1000 \
tunnel-server
```

### 3. Enhanced Observability

New business/SLO metrics available:
- `mockforge_service_availability`
- `mockforge_slo_compliance`
- `mockforge_successful_request_rate`
- `mockforge_p95_latency_slo_compliance`
- `mockforge_error_budget_remaining`

**Migration**:
- Metrics are automatically exposed (no code changes needed)
- Update Grafana dashboards to include new metrics (optional)

### 4. Security Scanning

Integrated security scanning:
```bash
# Run comprehensive security scan
make security-scan

# Quick security check
make security-check
```

## Deprecated Features

### None

MockForge 1.0 does not deprecate any major features. All 0.2.x features remain available.

## Configuration Changes

### Health Check Configuration

**New Options**:
```yaml
health:
  enabled: true
  init_timeout_seconds: 60  # New: startup timeout
  liveness_path: /health/live
  readiness_path: /health/ready
  startup_path: /health/startup
```

### Tunnel Server Configuration

**New Environment Variables**:
```bash
# Persistent storage
TUNNEL_DATABASE_PATH=/var/lib/mockforge/tunnels.db

# TLS
TUNNEL_TLS_CERT=/path/to/cert.pem
TUNNEL_TLS_KEY=/path/to/key.pem

# Rate limiting
TUNNEL_RATE_LIMIT_ENABLED=true
TUNNEL_RATE_LIMIT_RPM=1000
TUNNEL_RATE_LIMIT_PER_IP_RPM=100

# Audit logging
TUNNEL_AUDIT_LOG_ENABLED=true
TUNNEL_AUDIT_LOG_PATH=/var/log/mockforge/audit.log
```

## Step-by-Step Migration

### 1. Review Breaking Changes

- [ ] Review health check endpoints
- [ ] Review configuration files
- [ ] Review plugin code (if applicable)

### 2. Update Dependencies

```bash
# Update Cargo.toml
mockforge-core = "1.0"
mockforge-http = "1.0"
# ... other dependencies
```

### 3. Run Tests

```bash
# Run full test suite
cargo test --workspace

# Run integration tests
make test-integration
```

### 4. Update Configuration

- [ ] Review configuration files
- [ ] Update Kubernetes deployments (health checks)
- [ ] Update tunnel server configuration (if using)

### 5. Deploy and Monitor

- [ ] Deploy to staging environment
- [ ] Monitor health check endpoints
- [ ] Verify metrics collection
- [ ] Check logs for errors

### 6. Production Deployment

- [ ] Deploy to production
- [ ] Monitor for issues
- [ ] Collect feedback

## Rollback Plan

If you need to rollback:

1. **Dependencies**: Revert to 0.2.x in Cargo.toml
2. **Configuration**: Revert configuration changes
3. **Deployment**: Revert to previous deployment

**Note**: MockForge 1.0 is designed to be backwards compatible where possible. Most deployments should work without changes.

## Troubleshooting

### Health Checks Failing

**Problem**: Kubernetes health checks failing after upgrade.

**Solution**: Update health check paths to use `/health/ready` and `/health/live`.

### Configuration Errors

**Problem**: Configuration file not loading.

**Solution**:
1. Check configuration syntax
2. Review configuration documentation
3. Validate with `mockforge config validate`

### Plugin Issues

**Problem**: Plugins not loading.

**Solution**:
1. Check plugin compatibility
2. Review plugin documentation
3. Update plugins if needed

## Getting Help

- **Documentation**: https://docs.mockforge.dev
- **Issues**: https://github.com/SaaSy-Solutions/mockforge/issues
- **Discussions**: https://github.com/SaaSy-Solutions/mockforge/discussions

## Summary

MockForge 1.0 is designed to be a smooth upgrade from 0.2.x:
- ✅ Most features are backwards compatible
- ✅ Breaking changes are minimal and well-documented
- ✅ Migration path is straightforward
- ✅ New features are optional to adopt

**Estimated Migration Time**: 1-2 hours for typical deployments.

---

**Last Updated**: 2025-01-27
**Version**: 1.0.0
