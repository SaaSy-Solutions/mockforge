# Plugin Commands - Quick Reference

## 🚀 Installation

```bash
# From URL
mockforge plugin install https://example.com/plugin.zip

# From GitHub (latest)
mockforge plugin install https://github.com/user/repo

# From GitHub (specific version)
mockforge plugin install https://github.com/user/repo#v1.0.0

# From local path
mockforge plugin install ./my-plugin

# With checksum verification
mockforge plugin install https://example.com/plugin.zip --checksum abc123...

# Force reinstall
mockforge plugin install <source> --force
```

## 📋 Management

```bash
# List installed plugins
mockforge plugin list

# Show detailed info
mockforge plugin list --detailed

# Get plugin info
mockforge plugin info <plugin-id>

# Uninstall plugin
mockforge plugin uninstall <plugin-id>

# Update plugin
mockforge plugin update <plugin-id>

# Update all plugins
mockforge plugin update --all
```

## 🔍 Validation

```bash
# Validate without installing
mockforge plugin validate <source>
```

## 💾 Cache

```bash
# Show cache statistics
mockforge plugin cache-stats

# Clear cache
mockforge plugin clear-cache

# Clear cache and show stats
mockforge plugin clear-cache --stats
```

## 🔍 Search (Coming in Phase 3)

```bash
# Search marketplace
mockforge plugin search auth

# Search with filters
mockforge plugin search auth --category security --limit 20
```

## 🔗 Source Formats

| Format | Example |
|--------|---------|
| **Direct URL** | `https://example.com/plugin.zip` |
| **GitHub HTTPS** | `https://github.com/user/repo#v1.0.0` |
| **GitHub SSH** | `git@github.com:user/repo.git#v1.0.0` |
| **Local path** | `./my-plugin` or `/path/to/plugin` |
| **Registry** | `auth-jwt@1.0.0` (Phase 3) |
| **With subdirectory** | `https://github.com/user/repo#main:plugins/auth` |

## 🎯 Common Workflows

### Development Workflow
```bash
# Install local plugin for development
mockforge plugin install ./my-plugin-dev

# Make changes to plugin code

# Reinstall to test
mockforge plugin install ./my-plugin-dev --force
```

### Production Deployment
```bash
# Install with version pinning
mockforge plugin install https://github.com/company/plugin#v1.0.0 \
  --checksum e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855

# Verify installation
mockforge plugin list --detailed

# Test with MockForge
mockforge serve --config production.yaml
```

### Update Workflow
```bash
# Check for updates
mockforge plugin list

# Update specific plugin
mockforge plugin update auth-jwt

# Update all plugins
mockforge plugin update --all
```

## ⚙️ Options

### Install Options
- `--force` - Force reinstall even if exists
- `--skip-validation` - Skip validation (not recommended)
- `--no-verify` - Don't verify signature
- `--checksum <HASH>` - Expected SHA-256 checksum

### List Options
- `--detailed` - Show detailed information

### Cache Options
- `--stats` - Show statistics before clearing

## 🔧 Environment Variables

```bash
# Custom cache directory
export MOCKFORGE_PLUGIN_CACHE_DIR=/tmp/plugins

# Disable SSL verification (not recommended)
export MOCKFORGE_PLUGIN_VERIFY_SSL=false

# Custom timeout (seconds)
export MOCKFORGE_PLUGIN_TIMEOUT=600
```

## 🐛 Troubleshooting

### SSL Certificate Error
```bash
# Check URL is correct
curl -I <url>

# Verify certificate validity
openssl s_client -connect example.com:443
```

### Download Timeout
```bash
# Increase timeout
mockforge plugin install <url> --timeout 600
```

### Git Clone Failed
```bash
# For private repos, configure SSH
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_rsa

# Then install
mockforge plugin install git@github.com:company/plugin.git
```

### Plugin Already Installed
```bash
# Force reinstall
mockforge plugin install <source> --force
```

## 📚 More Information

- [Full Documentation](./remote-loading.md)
- [Plugin Development Guide](./development-guide.md)
- [Security Model](./security/model.md)
- [Example Plugins](../../examples/plugins/README.md)
