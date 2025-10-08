# MockForge Plugin Registry

Complete guide to using and operating the MockForge Plugin Registry.

## Overview

The MockForge Plugin Registry is a central repository for discovering, publishing, and installing MockForge plugins. It provides:

- **Plugin Discovery**: Search and browse available plugins
- **Version Management**: Install specific versions with dependency resolution
- **Publishing**: Share your plugins with the community
- **Security**: Checksum verification and authentication
- **Statistics**: Download counts, ratings, and reviews

## For Plugin Users

### Searching for Plugins

Search the registry for plugins:

```bash
# Basic search
mockforge plugin registry search auth

# Filter by category
mockforge plugin registry search --category auth

# Filter by tags
mockforge plugin registry search --tags jwt,oauth

# Sort results
mockforge plugin registry search --sort downloads
mockforge plugin registry search --sort rating
mockforge plugin registry search --sort recent

# Paginate results
mockforge plugin registry search --page 2 --per-page 50
```

### Installing from Registry

Install plugins directly from the registry:

```bash
# Install latest version
mockforge plugin registry install auth-jwt

# Install specific version
mockforge plugin registry install auth-jwt@1.2.0

# Force reinstall
mockforge plugin registry install auth-jwt --force
```

The registry installation automatically:
- Downloads the plugin package
- Verifies checksums
- Installs dependencies
- Validates compatibility

### Viewing Plugin Information

Get detailed information about a plugin:

```bash
# Show plugin details
mockforge plugin registry info auth-jwt

# Show specific version details
mockforge plugin registry info auth-jwt --version 1.2.0
```

Output includes:
- Plugin description
- Available versions
- Download statistics
- Rating and reviews
- License information
- Repository links

## For Plugin Developers

### Publishing Your Plugin

#### 1. Prepare Your Plugin

Ensure your `plugin.yaml` manifest is complete:

```yaml
name: my-awesome-plugin
version: 1.0.0
description: A fantastic plugin that does amazing things
author:
  name: Your Name
  email: you@example.com
license: MIT
repository: https://github.com/yourusername/my-awesome-plugin
tags:
  - auth
  - security
  - jwt
category: auth
```

#### 2. Login to Registry

Set your API token:

```bash
# Interactive login
mockforge plugin registry login

# Or provide token directly
mockforge plugin registry login --token YOUR_API_TOKEN
```

Get your API token from: https://registry.mockforge.dev/settings/tokens

#### 3. Publish Your Plugin

```bash
# Validate without publishing
mockforge plugin registry publish --dry-run

# Publish to registry
mockforge plugin registry publish
```

The publish process:
1. Validates your manifest
2. Builds the plugin (if needed)
3. Calculates checksums
4. Uploads to registry
5. Updates the index

#### 4. Managing Published Versions

Remove a version from the index (yank):

```bash
mockforge plugin registry yank my-plugin 1.0.0
```

**Note**: Yanking removes a version from the index but doesn't delete files. Users who already installed it can still use it.

### Publishing Best Practices

#### Version Numbering

Follow semantic versioning (semver):
- **MAJOR** (1.x.x): Breaking changes
- **MINOR** (x.1.x): New features, backwards compatible
- **PATCH** (x.x.1): Bug fixes

Example:
```
1.0.0 → Initial release
1.1.0 → Add new feature
1.1.1 → Fix bug
2.0.0 → Breaking change
```

#### Manifest Guidelines

**Required Fields**:
- `name`: Lowercase alphanumeric with hyphens (e.g., `my-auth-plugin`)
- `version`: Valid semver (e.g., `1.0.0`)
- `description`: Clear, concise (max 500 characters)
- `author.name`: Your name or organization
- `license`: Valid SPDX identifier

**Recommended Fields**:
- `repository`: Link to source code
- `homepage`: Plugin documentation
- `tags`: Descriptive keywords (max 10, max 20 chars each)
- `category`: Helps users find your plugin

#### Quality Checklist

Before publishing:

- [ ] Plugin builds without errors
- [ ] All tests pass
- [ ] Documentation is complete
- [ ] Examples are included
- [ ] CHANGELOG is updated
- [ ] License file exists
- [ ] README explains usage
- [ ] Semantic version is correct

## Registry Configuration

### Default Configuration

The registry uses these defaults:

```toml
# ~/.config/mockforge/registry.toml

url = "https://registry.mockforge.dev"
timeout = 30
token = "YOUR_TOKEN"  # Set via 'login' command

# Alternative registries (optional)
alternative_registries = [
    "https://private-registry.company.com"
]
```

### Custom Registry

Use a private or alternative registry:

```bash
# Set custom registry URL
mockforge plugin registry config --url https://my-registry.com

# Or via environment variable
export MOCKFORGE_REGISTRY_URL=https://my-registry.com
```

### View Configuration

```bash
mockforge plugin registry config
```

## Registry API

The registry exposes a REST API at `/api/v1/`:

### Search Plugins

```http
POST /api/v1/plugins/search
Content-Type: application/json

{
  "query": "auth",
  "category": "auth",
  "tags": ["jwt"],
  "sort": "downloads",
  "page": 0,
  "per_page": 20
}
```

### Get Plugin Details

```http
GET /api/v1/plugins/{name}
```

### Get Specific Version

```http
GET /api/v1/plugins/{name}/versions/{version}
```

### Publish Plugin

```http
POST /api/v1/plugins/publish
Authorization: Bearer {token}
Content-Type: application/json

{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "...",
  ...
}
```

### Yank Version

```http
DELETE /api/v1/plugins/{name}/versions/{version}/yank
Authorization: Bearer {token}
```

## Security

### Authentication

- **API Tokens**: Required for publishing and yanking
- **Token Scope**: Tokens are user-specific
- **Token Storage**: Stored locally in `~/.config/mockforge/registry.toml`

### Checksum Verification

All published plugins include SHA-256 checksums:

```json
{
  "version": "1.0.0",
  "checksum": "abc123...",
  "download_url": "https://..."
}
```

The CLI automatically verifies checksums after download.

### Permission Model

- **Public**: Anyone can search and download
- **Authenticated**: Token required to publish or yank
- **Owner**: Only plugin owners can publish updates or yank versions

## Self-Hosting

### Running Your Own Registry

Deploy a private registry server:

```bash
# Using Docker
docker run -d -p 8080:8080 \
  -v /data/plugins:/var/lib/mockforge-registry \
  mockforge/registry:latest

# Using cargo
cargo install mockforge-registry-server
mockforge-registry-server --host 0.0.0.0 --port 8080
```

### Configure Clients

Point clients to your registry:

```bash
mockforge plugin registry config --url http://localhost:8080
```

Or set globally:

```bash
export MOCKFORGE_REGISTRY_URL=http://localhost:8080
```

## Troubleshooting

### Common Issues

#### "Not logged in" Error

**Solution**: Run `mockforge plugin registry login`

#### "Plugin not found" Error

**Causes**:
- Plugin name misspelled
- Plugin not yet published
- Using wrong registry URL

**Solution**: Verify plugin name with `mockforge plugin registry search`

#### "Version not found" Error

**Causes**:
- Version doesn't exist
- Version was yanked

**Solution**: Check available versions with `mockforge plugin registry info <name>`

#### Publish Failed

**Causes**:
- Invalid manifest
- Version already published
- Missing authentication

**Solution**:
1. Validate with `--dry-run`
2. Check version in `plugin.yaml`
3. Verify login with `mockforge plugin registry config`

### Debug Mode

Enable verbose logging:

```bash
RUST_LOG=debug mockforge plugin registry search auth
```

## Statistics

View registry-wide statistics:

```bash
mockforge plugin registry stats
```

Shows:
- Total plugins
- Total downloads
- Most popular plugins
- Recent additions
- Categories distribution

## Plugin Categories

Plugins are organized into categories:

- **auth**: Authentication and authorization
- **template**: Template functions and helpers
- **response**: Response transformations
- **datasource**: External data integration
- **middleware**: Request/response middleware
- **testing**: Testing utilities
- **observability**: Monitoring and logging
- **other**: Uncategorized plugins

## Migration Guide

### From Manual Installation

If you're using manually installed plugins:

1. Check if plugin is in registry:
   ```bash
   mockforge plugin registry search <plugin-name>
   ```

2. Install from registry:
   ```bash
   mockforge plugin registry install <plugin-name>
   ```

3. Remove manual installation:
   ```bash
   mockforge plugin uninstall <old-plugin-id>
   ```

### Publishing Existing Plugins

To publish an existing plugin:

1. Add registry metadata to `plugin.yaml`
2. Run `mockforge plugin registry publish --dry-run`
3. Fix any validation errors
4. Publish: `mockforge plugin registry publish`

## Community Guidelines

### Plugin Naming

- Use descriptive names
- Lowercase with hyphens
- Include category prefix (e.g., `auth-jwt`, `template-crypto`)
- Avoid generic names

### Documentation

Include in your repository:
- `README.md`: Usage guide
- `CHANGELOG.md`: Version history
- `LICENSE`: License file
- `examples/`: Usage examples

### Support

Provide support channels:
- GitHub Issues
- Discussion forum
- Documentation site
- Email contact

## FAQ

**Q: How do I update a published plugin?**

A: Increment the version in `plugin.yaml` and publish again.

**Q: Can I delete a published version?**

A: No, but you can yank it to remove from the index.

**Q: How do I report a malicious plugin?**

A: Email security@mockforge.dev with details.

**Q: Can I use multiple registries?**

A: Yes, configure alternative registries in `registry.toml`.

**Q: Is there a size limit for plugins?**

A: Yes, 50MB per plugin package.

**Q: How often is the index updated?**

A: Instantly when plugins are published.

## Resources

- **Registry Website**: https://registry.mockforge.dev
- **API Documentation**: https://docs.mockforge.dev/registry/api
- **Plugin Development**: https://docs.mockforge.dev/plugins/development
- **Support**: https://github.com/mockforge/mockforge/discussions

## Contributing

Help improve the registry:

- Report bugs
- Suggest features
- Improve documentation
- Contribute code

Visit: https://github.com/mockforge/mockforge-registry
