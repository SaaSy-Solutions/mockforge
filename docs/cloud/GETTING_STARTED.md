# Getting Started with MockForge Cloud

Welcome to MockForge Cloud! This guide will help you get up and running quickly.

## What is MockForge Cloud?

MockForge Cloud is the hosted version of MockForge, providing:

- **Hosted Mock Services**: Deploy mocks to the cloud with shareable URLs
- **Plugin Marketplace**: Discover and install WASM plugins
- **Template Marketplace**: Use pre-built chaos orchestration templates
- **Scenario Marketplace**: Share complete mock configurations
- **Team Collaboration**: Work together with your team
- **Usage Analytics**: Track your API usage and performance

## Quick Start

### 1. Create an Account

1. Visit [app.mockforge.dev](https://app.mockforge.dev)
2. Click **Sign Up**
3. Choose your sign-up method:
   - **Email/Password**: Traditional account creation
   - **GitHub OAuth**: Sign in with your GitHub account
   - **Google OAuth**: Sign in with your Google account

### 2. Choose Your Plan

MockForge Cloud offers three plans:

#### Free Plan
- ✅ 10,000 requests per month
- ✅ 1 GB storage
- ✅ BYOK (Bring Your Own Key) for AI features
- ✅ Basic plugin marketplace access
- ✅ Community support

#### Pro Plan ($29/month)
- ✅ 100,000 requests per month
- ✅ 10 GB storage
- ✅ Hosted AI features (no BYOK required)
- ✅ Hosted mock deployments
- ✅ Priority support (48-hour SLA)
- ✅ Advanced analytics

#### Team Plan ($99/month)
- ✅ 1,000,000 requests per month
- ✅ 100 GB storage
- ✅ Everything in Pro
- ✅ SSO support
- ✅ Team collaboration features
- ✅ Dedicated support (24-hour SLA)

### 3. Set Up Your Organization

After signing up, you'll automatically have a personal organization. To create a team organization:

1. Go to **Organizations** in the sidebar
2. Click **Create Organization**
3. Enter organization name and slug
4. Invite team members (Team plan required)

### 4. Configure API Access

#### Option A: Personal Access Token (Recommended for CLI)

1. Go to **Settings** → **API Tokens**
2. Click **Create Token**
3. Give it a descriptive name (e.g., "Local Development")
4. Select scopes (read, write, admin)
5. Copy the token (you won't see it again!)

Use the token in your CLI:

```bash
# Set token in environment
export MOCKFORGE_TOKEN="mfx_..."

# Or use in CLI commands
mockforge registry login --token mfx_...
```

#### Option B: JWT Token (For Web UI)

The web UI automatically uses JWT tokens. No configuration needed!

### 5. Deploy Your First Hosted Mock

#### Using the Web UI

1. Go to **Hosted Mocks** in the sidebar
2. Click **Create Deployment**
3. Fill in:
   - **Name**: My API Mock
   - **Slug**: my-api-mock (used in URL)
   - **Description**: Optional description
   - **OpenAPI Spec URL**: URL to your OpenAPI spec
   - **Config JSON**: Optional mock configuration
4. Click **Deploy**
5. Your mock will be available at: `https://{org-slug}.mockforge.dev/{slug}`

#### Using the CLI

```bash
# First, authenticate
mockforge registry login

# Set your organization context
mockforge org use my-org

# Deploy a mock
mockforge deploy \
  --name "My API Mock" \
  --slug "my-api-mock" \
  --openapi-url "https://example.com/openapi.json"
```

### 6. Browse the Marketplace

#### Plugin Marketplace

1. Go to **Plugin Registry** in the sidebar
2. Browse available plugins
3. Click **Install** on any plugin
4. Use in your mocks via the plugin system

#### Template Marketplace

1. Go to **Template Marketplace** in the sidebar
2. Search for templates by category
3. Click **Use Template** to apply to your project
4. Customize as needed

#### Scenario Marketplace

1. Go to **Scenario Marketplace** in the sidebar
2. Browse complete mock scenarios
3. Click **Use Scenario** to import
4. Deploy or modify as needed

### 7. Publish Your Own Content

#### Publish a Plugin

**Via CLI:**
```bash
mockforge plugin publish \
  --name "my-plugin" \
  --version "1.0.0" \
  --wasm-file "./plugin.wasm"
```

**Via UI:**
1. Go to **Plugin Registry**
2. Click **Publish Plugin**
3. Fill in metadata and upload WASM file
4. Click **Publish**

#### Publish a Template

**Via CLI:**
```bash
mockforge template publish \
  --manifest "./template-manifest.json" \
  --package "./template.tar.gz"
```

**Via UI:**
1. Go to **Template Marketplace**
2. Click **Publish Template**
3. Fill in metadata and upload package
4. Click **Publish**

#### Publish a Scenario

**Via CLI:**
```bash
mockforge scenario publish \
  --name "my-scenario" \
  --version "1.0.0" \
  --manifest "./scenario-manifest.json"
```

**Via UI:**
1. Go to **Scenario Marketplace**
2. Click **Publish Scenario**
3. Fill in metadata and upload package
4. Click **Publish**

## Key Concepts

### Organizations

Organizations are the primary way to organize your work in MockForge Cloud:

- **Personal Organization**: Automatically created for each user
- **Team Organizations**: Created for teams (Team plan required)
- **Organization Context**: Determines which org's resources you're working with

Switch organization context:
```bash
# List organizations
mockforge org list

# Use a specific organization
mockforge org use my-team-org

# Check current organization
mockforge org current
```

### Projects

Projects are containers for your mocks within an organization:

- Each organization can have multiple projects
- Projects can be public or private
- Projects have their own environments (dev, staging, prod)

### Hosted Mocks

Hosted mocks are deployed mock services accessible via URLs:

- **URL Format**: `https://{org-slug}.mockforge.dev/{deployment-slug}`
- **Automatic Scaling**: Handled by MockForge Cloud
- **Health Monitoring**: Automatic health checks every 30 seconds
- **Metrics**: Collected every minute

### Usage and Limits

Track your usage in the **Usage Dashboard**:

- **Requests**: API calls to your hosted mocks
- **Storage**: Data stored in MockForge Cloud
- **AI Tokens**: AI feature usage (Free plan requires BYOK)

View limits and current usage:
```bash
# Via CLI
mockforge usage

# Via UI
Go to Usage Dashboard
```

## Common Workflows

### Workflow 1: Deploy a Mock from OpenAPI Spec

1. **Prepare your OpenAPI spec** (JSON or YAML)
2. **Deploy via UI or CLI**:
   ```bash
   mockforge deploy \
     --name "User API" \
     --slug "user-api" \
     --openapi-url "https://api.example.com/openapi.json"
   ```
3. **Access your mock**: `https://my-org.mockforge.dev/user-api`
4. **Test endpoints**: Use the mock URL in your tests

### Workflow 2: Use a Template for Chaos Testing

1. **Browse Template Marketplace**
2. **Select a template** (e.g., "API Resilience Test")
3. **Apply to your project**
4. **Customize parameters**
5. **Run the orchestration**
6. **Review results** in the analytics dashboard

### Workflow 3: Share a Scenario with Your Team

1. **Create a scenario** locally
2. **Package it**:
   ```bash
   mockforge scenario package \
     --name "payment-flow" \
     --version "1.0.0"
   ```
3. **Publish to marketplace**:
   ```bash
   mockforge scenario publish \
     --name "payment-flow" \
     --version "1.0.0"
   ```
4. **Team members can now use it** from the Scenario Marketplace

### Workflow 4: Set Up BYOK for AI Features (Free Plan)

1. **Go to Settings** → **BYOK Configuration**
2. **Enter your OpenAI API key** (or other provider)
3. **Save configuration**
4. **AI features now use your key**

Note: Pro and Team plans include hosted AI features - no BYOK needed!

## Migration from Local to Cloud

If you're already using MockForge locally:

### Step 1: Export Your Configuration

```bash
# Export your local configuration
mockforge config export > local-config.json
```

### Step 2: Create Cloud Account

Sign up at [app.mockforge.dev](https://app.mockforge.dev)

### Step 3: Import Configuration

```bash
# Authenticate with cloud
mockforge registry login

# Import configuration
mockforge config import < local-config.json
```

### Step 4: Deploy to Cloud

```bash
# Deploy your mocks
mockforge deploy --from-config local-config.json
```

See [Migration Guide](MIGRATION_GUIDE_LOCAL_TO_CLOUD.md) for detailed migration instructions.

## CLI Commands Reference

### Authentication

```bash
# Login with OAuth (opens browser)
mockforge registry login

# Login with token
mockforge registry login --token mfx_...

# Logout
mockforge registry logout
```

### Organization Management

```bash
# List organizations
mockforge org list

# Use organization
mockforge org use my-org

# Show current organization
mockforge org current

# Clear organization context
mockforge org clear
```

### Deployment

```bash
# Create deployment
mockforge deploy \
  --name "My Mock" \
  --slug "my-mock" \
  --openapi-url "https://..."

# List deployments
mockforge deployments list

# Get deployment status
mockforge deployments status <deployment-id>

# View logs
mockforge deployments logs <deployment-id>

# Delete deployment
mockforge deployments delete <deployment-id>
```

### Marketplace

```bash
# Search plugins
mockforge plugin search --query "http"

# Install plugin
mockforge plugin install <plugin-name>

# Publish plugin
mockforge plugin publish --name "..." --version "..." --wasm-file "..."

# Search templates
mockforge template search --category "chaos"

# Publish template
mockforge template publish --manifest "..." --package "..."

# Publish scenario
mockforge scenario publish --name "..." --version "..." --manifest "..."
```

### Usage

```bash
# View current usage
mockforge usage

# View usage history
mockforge usage history
```

## Troubleshooting

### "Authentication required" error

**Solution**: Make sure you're logged in:
```bash
mockforge registry login
```

### "Organization not found" error

**Solution**: Set your organization context:
```bash
mockforge org use <org-slug>
```

### Deployment fails

**Common causes**:
- Invalid OpenAPI spec URL
- OpenAPI spec not accessible
- Invalid configuration JSON

**Solution**: Check deployment logs:
```bash
mockforge deployments logs <deployment-id>
```

### Rate limit exceeded

**Solution**:
- Check your usage: `mockforge usage`
- Upgrade your plan if needed
- Wait for the rate limit window to reset

### Email notifications not working

**Solution**:
- Check your email provider settings (Postmark/Brevo)
- Verify `EMAIL_PROVIDER` and `EMAIL_API_KEY` environment variables
- Check server logs for email errors

## Support

Need help? We're here for you:

- **Support Page**: Submit a request in-app
- **Email**: support@mockforge.dev
- **Documentation**: [docs.mockforge.dev](https://docs.mockforge.dev)
- **GitHub**: [github.com/SaaSy-Solutions/mockforge](https://github.com/SaaSy-Solutions/mockforge)

**Response Times**:
- **Free**: Best effort
- **Pro**: 48 hours
- **Team**: 24 hours

## Next Steps

Now that you're set up, explore:

1. **Advanced Features**:
   - Set up team collaboration
   - Configure BYOK for AI features
   - Explore analytics dashboard

2. **Marketplace**:
   - Browse and install plugins
   - Use templates for common scenarios
   - Share your own content

3. **Integration**:
   - Integrate with CI/CD pipelines
   - Use in automated testing
   - Connect to monitoring tools

4. **Documentation**:
   - [API Reference](../api-reference.md)
   - [Plugin Development Guide](../plugin-development.md)
   - [Template Creation Guide](../template-creation.md)

Welcome to MockForge Cloud! 🚀
