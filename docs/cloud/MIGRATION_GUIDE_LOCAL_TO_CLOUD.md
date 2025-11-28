# Migration Guide: Local MockForge to MockForge Cloud

This guide helps you migrate from running MockForge locally to using MockForge Cloud, ensuring a smooth transition with minimal downtime.

## Table of Contents

- [Why Migrate?](#why-migrate)
- [Pre-Migration Checklist](#pre-migration-checklist)
- [Migration Process](#migration-process)
- [Configuration Mapping](#configuration-mapping)
- [Data Migration](#data-migration)
- [Testing Your Migration](#testing-your-migration)
- [Post-Migration](#post-migration)
- [Rollback Plan](#rollback-plan)
- [Common Issues](#common-issues)
- [FAQ](#faq)

---

## Why Migrate?

### Benefits of MockForge Cloud

- **No Infrastructure Management**: No need to run servers, manage updates, or handle scaling
- **Public URLs**: Share mocks with team members and external services without VPNs or tunnels
- **Team Collaboration**: Multiple team members can work on the same mocks
- **Marketplace Access**: Discover and use plugins, templates, and scenarios from the community
- **Usage Analytics**: Track API usage, performance metrics, and costs
- **Automatic Scaling**: Handles traffic spikes automatically
- **High Availability**: 99.9% uptime SLA (Pro/Team plans)
- **Backup & Recovery**: Automatic backups and point-in-time recovery

### When to Stay Local

You might want to keep running locally if:
- You have strict data residency requirements
- You need complete control over infrastructure
- You have very high traffic (cost considerations)
- You're in a highly regulated industry with compliance needs

---

## Pre-Migration Checklist

Before starting your migration, ensure you have:

- [ ] **Inventory of Local Mocks**
  - List all mock servers you're running
  - Document their configurations
  - Note any custom plugins or templates

- [ ] **Configuration Files**
  - Collect all `mockforge.yaml` files
  - Document environment variables
  - Note any custom settings

- [ ] **OpenAPI Specs**
  - Locate all OpenAPI/Swagger specs
  - Verify they're accessible (URLs or file paths)
  - Check for any private/internal dependencies

- [ ] **Custom Plugins**
  - List all custom WASM plugins
  - Ensure plugin source code is available
  - Document plugin dependencies

- [ ] **Templates & Scenarios**
  - Inventory all custom templates
  - List any scenario files
  - Document their use cases

- [ ] **Dependencies**
  - Note any external service dependencies
  - Document webhook endpoints
  - List any integrations

- [ ] **Usage Patterns**
  - Understand current traffic patterns
  - Estimate monthly request volume
  - Identify peak usage times

- [ ] **Team Access**
  - List team members who need access
  - Determine organization structure
  - Plan for team collaboration

---

## Migration Process

### Step 1: Create Cloud Account

1. **Sign Up**: Visit [app.mockforge.dev](https://app.mockforge.dev) and create an account
2. **Choose Plan**: Select a plan based on your usage estimates
   - **Free**: Good for testing and small projects
   - **Pro**: Recommended for production use
   - **Team**: Best for multiple team members
3. **Verify Email**: Complete email verification
4. **Create Organization**: Set up your organization (or use the default personal org)

### Step 2: Install/Update CLI

Ensure you have the latest MockForge CLI:

```bash
# Update CLI
cargo install mockforge-cli --force

# Or via package manager
# Check your package manager for latest version
```

Verify installation:
```bash
mockforge --version
```

### Step 3: Authenticate with Cloud

```bash
# Login with OAuth (opens browser)
mockforge registry login

# Or login with API token
mockforge registry login --token mfx_...
```

Verify authentication:
```bash
mockforge org list
```

### Step 4: Set Organization Context

```bash
# List available organizations
mockforge org list

# Set your organization
mockforge org use my-org

# Verify current organization
mockforge org current
```

### Step 5: Export Local Configuration

For each mock server you're running locally:

```bash
# Export configuration to JSON
mockforge config export > local-mock-config.json

# Or manually document your mockforge.yaml settings
cat mockforge.yaml > local-config-backup.yaml
```

### Step 6: Migrate Mocks

#### Option A: Migrate via CLI (Recommended)

For each mock:

```bash
# Deploy from OpenAPI spec
mockforge deploy \
  --name "User API" \
  --slug "user-api" \
  --openapi-url "https://example.com/openapi.json" \
  --config-file local-mock-config.json

# Or deploy from local config
mockforge deploy \
  --name "User API" \
  --slug "user-api" \
  --from-config local-mock-config.json
```

#### Option B: Migrate via UI

1. Go to **Hosted Mocks** in the sidebar
2. Click **Create Deployment**
3. Fill in:
   - **Name**: Descriptive name
   - **Slug**: URL-friendly identifier
   - **OpenAPI Spec URL**: URL to your spec (or upload)
   - **Config JSON**: Paste your configuration JSON
4. Click **Deploy**

### Step 7: Migrate Custom Plugins

If you have custom plugins:

```bash
# Publish each plugin
mockforge plugin publish \
  --name "my-custom-plugin" \
  --version "1.0.0" \
  --wasm-file "./plugin.wasm" \
  --description "Custom authentication plugin"
```

Or via UI:
1. Go to **Plugin Registry**
2. Click **Publish Plugin**
3. Upload WASM file and fill in metadata

### Step 8: Migrate Templates & Scenarios

```bash
# Publish template
mockforge template publish \
  --name "my-template" \
  --version "1.0.0" \
  --manifest "./template-manifest.json" \
  --package "./template.tar.gz"

# Publish scenario
mockforge scenario publish \
  --name "my-scenario" \
  --version "1.0.0" \
  --manifest "./scenario-manifest.json" \
  --package "./scenario.tar.gz"
```

### Step 9: Update Integration Points

Update all places that reference your local mocks:

#### Update Environment Variables

```bash
# Old (local)
export API_BASE_URL="http://localhost:3000"

# New (cloud)
export API_BASE_URL="https://my-org.mockforge.dev/user-api"
```

#### Update Test Files

```typescript
// Old (local)
const mockServer = 'http://localhost:3000';

// New (cloud)
const mockServer = 'https://my-org.mockforge.dev/user-api';
```

#### Update CI/CD Pipelines

```yaml
# Old (local)
- name: Start Mock Server
  run: mockforge serve --config mockforge.yaml

# New (cloud)
- name: Use Cloud Mock
  env:
    API_BASE_URL: https://my-org.mockforge.dev/user-api
```

#### Update Documentation

Update any documentation that references local endpoints.

---

## Configuration Mapping

### Local Configuration → Cloud Configuration

| Local Setting | Cloud Equivalent | Notes |
|--------------|------------------|-------|
| `http.port` | N/A | Cloud handles port assignment |
| `http.host` | N/A | Cloud provides public URL |
| `http.openapi_spec` | Deployment OpenAPI URL | Upload spec or provide URL |
| `admin.enabled` | Always enabled | Admin UI always available |
| `admin.port` | N/A | Access via web UI |
| `logging.level` | N/A | Cloud handles logging |
| `plugins` | Plugin Registry | Install from marketplace |
| `templates` | Template Marketplace | Use from marketplace |
| Environment variables | Organization Settings | Store in org settings |

### Example Migration

**Local `mockforge.yaml`:**
```yaml
http:
  port: 3000
  openapi_spec: "./api-spec.json"
  cors_enabled: true

admin:
  enabled: true
  port: 9080

plugins:
  - name: "auth-plugin"
    path: "./plugins/auth.wasm"

logging:
  level: "info"
```

**Cloud Deployment:**
```bash
mockforge deploy \
  --name "User API" \
  --slug "user-api" \
  --openapi-url "https://example.com/api-spec.json" \
  --config '{"cors_enabled": true}'

# Install plugin from marketplace
mockforge plugin install auth-plugin
```

---

## Data Migration

### Mock Configurations

**Export local configs:**
```bash
# For each mock
mockforge config export --config mockforge.yaml > mock-1-config.json
```

**Import to cloud:**
```bash
# Deploy with config
mockforge deploy \
  --name "Mock 1" \
  --slug "mock-1" \
  --from-config mock-1-config.json
```

### Custom Plugins

1. **Package plugins:**
   ```bash
   # Ensure WASM files are available
   ls plugins/*.wasm
   ```

2. **Publish to marketplace:**
   ```bash
   mockforge plugin publish \
     --name "my-plugin" \
     --version "1.0.0" \
     --wasm-file "./plugins/my-plugin.wasm"
   ```

3. **Install in cloud:**
   ```bash
   mockforge plugin install my-plugin
   ```

### Templates & Scenarios

1. **Package templates:**
   ```bash
   tar -czf template.tar.gz template-manifest.json template-files/
   ```

2. **Publish:**
   ```bash
   mockforge template publish \
     --name "my-template" \
     --version "1.0.0" \
     --manifest "./template-manifest.json" \
     --package "./template.tar.gz"
   ```

### Fixtures & Test Data

If you have fixture files:

1. **Upload to object storage** (S3/B2) or make accessible via URL
2. **Reference in deployment config:**
   ```json
   {
     "fixtures": {
       "base_url": "https://my-storage.example.com/fixtures"
     }
   }
   ```

---

## Testing Your Migration

### 1. Verify Deployments

```bash
# List all deployments
mockforge deployments list

# Check deployment status
mockforge deployments status <deployment-id>

# View deployment logs
mockforge deployments logs <deployment-id>
```

### 2. Test Endpoints

```bash
# Test each endpoint
curl https://my-org.mockforge.dev/user-api/users/123

# Compare with local response
curl http://localhost:3000/users/123
```

### 3. Test Plugins

```bash
# Verify plugins are installed
mockforge plugin list

# Test plugin functionality
# (Use your test suite)
```

### 4. Load Testing

```bash
# Run load tests against cloud deployment
# Compare performance with local
```

### 5. Integration Testing

Run your full test suite against cloud mocks to ensure everything works.

---

## Post-Migration

### 1. Update Team Access

Invite team members to your organization:

1. Go to **Organizations** → **Members**
2. Click **Invite Member**
3. Enter email and assign role (Admin/Member)

### 2. Set Up Monitoring

- **Usage Dashboard**: Monitor API usage and costs
- **Deployment Metrics**: Track performance and errors
- **Alerts**: Set up alerts for failures or high usage

### 3. Configure BYOK (Free Plan)

If on Free plan and using AI features:

1. Go to **Settings** → **BYOK Configuration**
2. Enter your OpenAI (or other provider) API key
3. Save configuration

### 4. Set Up CI/CD Integration

Update your CI/CD pipelines to use cloud mocks:

```yaml
# GitHub Actions example
- name: Run Tests
  env:
    API_BASE_URL: https://my-org.mockforge.dev/user-api
  run: npm test
```

### 5. Document New URLs

Update all documentation with new cloud URLs:
- API documentation
- Test documentation
- Team wikis
- Integration guides

---

## Rollback Plan

If you need to rollback to local:

### Quick Rollback

1. **Keep local servers running** during migration
2. **Use feature flags** to switch between local and cloud
3. **Gradually migrate** one mock at a time

### Full Rollback

1. **Stop cloud deployments:**
   ```bash
   mockforge deployments delete <deployment-id>
   ```

2. **Restart local servers:**
   ```bash
   mockforge serve --config mockforge.yaml
   ```

3. **Update environment variables** back to local URLs

4. **Revert code changes** that reference cloud URLs

---

## Common Issues

### Issue: "Organization not found"

**Solution:**
```bash
# Set organization context
mockforge org use <org-slug>

# Verify
mockforge org current
```

### Issue: "Authentication required"

**Solution:**
```bash
# Re-authenticate
mockforge registry login

# Or use API token
mockforge registry login --token mfx_...
```

### Issue: "Deployment failed"

**Common causes:**
- Invalid OpenAPI spec URL
- Spec not accessible from cloud
- Invalid configuration JSON

**Solution:**
```bash
# Check deployment logs
mockforge deployments logs <deployment-id>

# Verify OpenAPI spec is accessible
curl https://your-spec-url.com/openapi.json

# Validate configuration
mockforge config validate --config config.json
```

### Issue: "Rate limit exceeded"

**Solution:**
- Check usage: `mockforge usage`
- Upgrade plan if needed
- Wait for rate limit window to reset
- Consider caching responses

### Issue: "Plugin not found"

**Solution:**
- Verify plugin is published: `mockforge plugin search <name>`
- Check organization context: `mockforge org current`
- Re-publish plugin if needed

### Issue: "Different response format"

**Solution:**
- Compare local vs cloud responses
- Check configuration differences
- Verify OpenAPI spec matches
- Check plugin compatibility

---

## FAQ

### Can I run both local and cloud simultaneously?

**Yes!** You can run local MockForge alongside cloud deployments. This is useful for:
- Gradual migration
- Testing before full migration
- Hybrid setups (some mocks local, some cloud)

### Will my local configuration work in cloud?

**Mostly yes**, but some differences:
- Port/host settings are ignored (cloud handles these)
- File paths need to be URLs or uploaded
- Local plugins need to be published to marketplace
- Admin UI is always available (no port config needed)

### How do I migrate custom plugins?

1. Ensure WASM file is available
2. Publish to marketplace: `mockforge plugin publish`
3. Install in cloud: `mockforge plugin install`
4. Reference in deployment config

### Can I keep my local setup as backup?

**Absolutely!** We recommend:
- Keeping local configs backed up
- Running local in parallel during migration
- Having a rollback plan ready

### What happens to my local data?

**Nothing!** Your local setup remains unchanged. Cloud is a separate service. You can:
- Keep local running
- Use both simultaneously
- Migrate gradually

### How do I handle private OpenAPI specs?

**Options:**
1. **Upload to cloud storage** (S3/B2) and use URL
2. **Make spec publicly accessible** (temporarily for migration)
3. **Use private cloud storage** with signed URLs
4. **Embed spec in deployment config** (for small specs)

### Can I migrate Docker setups?

**Yes!** Extract configuration from Docker:
```bash
# Export config from running container
docker exec mockforge cat /app/mockforge.yaml > config.yaml

# Use config for cloud deployment
mockforge deploy --from-config config.yaml
```

### What about environment-specific configs?

**Cloud supports:**
- **Projects**: Organize mocks by project
- **Environments**: Dev, staging, prod deployments
- **Organization Settings**: Store environment variables

Example:
```bash
# Deploy to different projects
mockforge deploy --project dev --name "API Mock"
mockforge deploy --project prod --name "API Mock"
```

### How do I migrate WebSocket mocks?

WebSocket mocks are deployed as hosted mocks:
```bash
mockforge deploy \
  --name "WebSocket Mock" \
  --slug "ws-mock" \
  --config '{"websocket": {"replay_file": "https://..."}}'
```

Access via: `wss://my-org.mockforge.dev/ws-mock`

### What about gRPC mocks?

gRPC mocks are also deployed as hosted mocks:
```bash
mockforge deploy \
  --name "gRPC Service" \
  --slug "grpc-service" \
  --config '{"grpc": {"proto_file": "https://..."}}'
```

Access via: `https://my-org.mockforge.dev/grpc-service` (HTTP bridge)

---

## Migration Checklist

Use this checklist to track your migration:

### Pre-Migration
- [ ] Inventory all local mocks
- [ ] Document configurations
- [ ] List custom plugins/templates
- [ ] Estimate usage and choose plan
- [ ] Create cloud account
- [ ] Set up organization

### Migration
- [ ] Authenticate CLI with cloud
- [ ] Set organization context
- [ ] Export local configurations
- [ ] Deploy first mock (test)
- [ ] Verify deployment works
- [ ] Deploy remaining mocks
- [ ] Publish custom plugins
- [ ] Publish templates/scenarios
- [ ] Update integration points
- [ ] Update documentation

### Post-Migration
- [ ] Test all endpoints
- [ ] Run full test suite
- [ ] Set up monitoring
- [ ] Configure team access
- [ ] Set up CI/CD integration
- [ ] Document new URLs
- [ ] Train team on cloud features

### Validation
- [ ] All mocks deployed successfully
- [ ] All endpoints responding correctly
- [ ] Plugins working as expected
- [ ] Performance meets requirements
- [ ] Team has access
- [ ] Monitoring configured
- [ ] Documentation updated

---

## Getting Help

If you encounter issues during migration:

1. **Check Logs**: `mockforge deployments logs <id>`
2. **Support Page**: Submit request in-app
3. **Email**: support@mockforge.dev
4. **Documentation**: [docs.mockforge.dev](https://docs.mockforge.dev)
5. **GitHub**: [github.com/SaaSy-Solutions/mockforge](https://github.com/SaaSy-Solutions/mockforge)

**Response Times:**
- Free: Best effort
- Pro: 48 hours
- Team: 24 hours

---

## Next Steps

After successful migration:

1. **Explore Marketplace**: Discover plugins, templates, and scenarios
2. **Set Up Team Collaboration**: Invite team members and assign roles
3. **Configure Analytics**: Set up usage tracking and alerts
4. **Optimize Costs**: Review usage and adjust plan if needed
5. **Share Knowledge**: Document your migration experience for your team

Welcome to MockForge Cloud! 🚀
