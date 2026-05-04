# Remote Plugin Loading

MockForge now supports loading plugins from remote sources, making it easy to share and install plugins from URLs, Git repositories, and future plugin registries.

## 🚀 Quick Start

### Install from URL

```bash
# Install from a direct download URL
mockforge plugin install https://example.com/plugins/auth-custom.zip

# Install with checksum verification
mockforge plugin install https://example.com/plugins/auth-custom.zip \
  --checksum abc123def456...
```

### Install from Git Repository

```bash
# Install from GitHub (default branch)
mockforge plugin install https://github.com/user/mockforge-plugin-auth

# Install a specific version (tag)
mockforge plugin install https://github.com/user/mockforge-plugin-auth#v1.0.0

# Install from a specific branch
mockforge plugin install https://github.com/user/mockforge-plugin-auth#develop

# Install from a specific commit
mockforge plugin install https://github.com/user/mockforge-plugin-auth#abc123def

# Install from a subdirectory in the repository
mockforge plugin install https://github.com/user/plugins#main:auth-plugin
```

### Install from Local Path

```bash
# Install from a local directory
mockforge plugin install ./path/to/plugin

# Install from an absolute path
mockforge plugin install /home/user/plugins/my-plugin
```

## 📋 Supported Plugin Sources

### 1. HTTP/HTTPS URLs

Download plugins from any web server:

**Supported Formats:**
- `.zip` - ZIP archives
- `.tar.gz` / `.tgz` - Compressed tar archives
- `.wasm` - Direct WebAssembly modules

**Example:**
```bash
mockforge plugin install https://cdn.example.com/plugins/my-plugin-v1.0.0.zip
```

**Security:**
- SSL certificate validation (enforced by default)
- Optional SHA-256 checksum verification
- Download size limits (100MB default)
- Progress tracking
- Automatic caching

### 2. Git Repositories

Clone plugins directly from Git:

**Supported Hosts:**
- GitHub
- GitLab
- Bitbucket
- Self-hosted Git servers
- Any Git URL (HTTPS or SSH)

**Version Pinning:**
```bash
# Latest from default branch
mockforge plugin install https://github.com/user/repo

# Specific tag
mockforge plugin install https://github.com/user/repo#v1.2.0

# Specific branch
mockforge plugin install https://github.com/user/repo#feature-branch

# Specific commit
mockforge plugin install https://github.com/user/repo#abc123def456

# Subdirectory within repository
mockforge plugin install https://github.com/user/repo#main:plugins/auth
```

**Features:**
- Shallow clones for performance
- Automatic caching
- Update support
- SSH and HTTPS authentication

### 3. Local File System

Install from local directories or files:

```bash
# Relative path
mockforge plugin install ./plugins/my-plugin

# Absolute path
mockforge plugin install /home/user/plugins/my-plugin

# Home directory (with tilde expansion)
mockforge plugin install ~/plugins/my-plugin
```

### 4. Plugin Registry (Coming Soon)

Install from the official MockForge plugin marketplace:

```bash
# Install latest version
mockforge plugin install auth-jwt

# Install specific version
mockforge plugin install auth-jwt@1.0.0

# Search for plugins
mockforge plugin search auth
```

## 🔒 Security Features

### Checksum Verification

Verify download integrity with SHA-256 checksums:

```bash
mockforge plugin install https://example.com/plugin.zip \
  --checksum abc123def456789...
```

The plugin will not be installed if the checksum doesn't match.

### SSL Certificate Validation

All HTTPS downloads validate SSL certificates. Self-signed certificates are rejected by default.

### Plugin Signature Verification

Verify plugin authenticity with GPG signatures:

```bash
# Verify signature (default)
mockforge plugin install https://example.com/plugin.zip

# Skip signature verification (not recommended)
mockforge plugin install https://example.com/plugin.zip --no-verify
```

**Note:** Plugins must include a `plugin.sig` file for verification.

### Sandboxing

All remote plugins are sandboxed with strict resource limits:
- Memory limits
- CPU time limits
- Restricted filesystem access
- Restricted network access

Capabilities must be explicitly declared in `plugin.yaml`.

## 📦 Caching

MockForge caches downloaded plugins to improve performance and reduce bandwidth usage.

### View Cache Statistics

```bash
mockforge plugin cache-stats
```

Output:
```
📊 Plugin cache statistics:
   Download cache: 45.23 MB
   Git cache: 120.45 MB
   Total: 165.68 MB
```

### Clear Cache

```bash
# Clear all caches
mockforge plugin clear-cache

# Show stats before clearing
mockforge plugin clear-cache --stats
```

Cache locations:
- **Linux**: `~/.cache/mockforge/plugins`
- **macOS**: `~/Library/Caches/mockforge/plugins`
- **Windows**: `%LOCALAPPDATA%\mockforge\plugins`

## 🛠️ Plugin Management Commands

### Install

```bash
mockforge plugin install <source> [OPTIONS]

Options:
  --force              Force reinstall even if plugin exists
  --skip-validation    Skip validation checks (not recommended)
  --no-verify          Don't verify plugin signature
  --checksum <HASH>    Expected SHA-256 checksum
```

### Uninstall

```bash
mockforge plugin uninstall <plugin-id>
```

### List Installed Plugins

```bash
# Simple list
mockforge plugin list

# Detailed information
mockforge plugin list --detailed
```

### Show Plugin Info

```bash
mockforge plugin info <plugin-id>
```

### Update Plugins

```bash
# Update specific plugin
mockforge plugin update <plugin-id>

# Update all plugins
mockforge plugin update --all
```

### Validate Plugin

Validate a plugin without installing:

```bash
mockforge plugin validate <source>
```

## ⚙️ Configuration

### Installation Options

Create a configuration file `~/.mockforge/config.yaml`:

```yaml
plugins:
  # Plugin directories to scan
  dirs:
    - "~/.mockforge/plugins"
    - "./plugins"

  # Remote download settings
  remote:
    max_download_size: 104857600  # 100MB
    timeout_seconds: 300          # 5 minutes
    max_retries: 3
    verify_ssl: true
    show_progress: true

  # Git clone settings
  git:
    shallow_clone: true
    include_submodules: false

  # Security settings
  security:
    verify_signatures: true
    allowed_sources:
      - "github.com"
      - "gitlab.com"
      - "plugins.mockforge.dev"
```

### Environment Variables

Override settings with environment variables:

```bash
# Disable SSL verification (not recommended)
export MOCKFORGE_PLUGIN_VERIFY_SSL=false

# Set custom cache directory
export MOCKFORGE_PLUGIN_CACHE_DIR=/tmp/mockforge-cache

# Set download timeout
export MOCKFORGE_PLUGIN_TIMEOUT=600
```

## 🔧 Troubleshooting

### Common Issues

#### 1. SSL Certificate Error

```
❌ Failed to download: SSL certificate problem
```

**Solution:** Ensure SSL certificates are valid. If using self-signed certificates:
```bash
# Not recommended for production
export MOCKFORGE_PLUGIN_VERIFY_SSL=false
```

#### 2. Download Timeout

```
❌ Download timeout after 300 seconds
```

**Solution:** Increase timeout:
```bash
mockforge plugin install <url> --timeout 600
```

#### 3. Checksum Mismatch

```
❌ Checksum verification failed
```

**Solution:** Verify the checksum is correct or omit the `--checksum` flag.

#### 4. Git Clone Failed

```
❌ Failed to clone repository
```

**Solutions:**
- Check repository URL
- Ensure you have access (for private repos)
- Configure SSH keys for SSH URLs
- Use HTTPS instead of SSH

#### 5. Plugin Already Installed

```
❌ Plugin already loaded: auth-jwt
```

**Solution:** Use `--force` to reinstall:
```bash
mockforge plugin install <source> --force
```

### Debug Mode

Enable verbose logging:

```bash
RUST_LOG=mockforge_plugin_loader=debug mockforge plugin install <source>
```

## 📚 Examples

### Example 1: Install Official Plugin

```bash
# Install from MockForge plugin repository
mockforge plugin install https://github.com/mockforge/plugins#main:auth-jwt
```

### Example 2: Install with Verification

```bash
# Install with checksum verification
mockforge plugin install https://example.com/plugin.zip \
  --checksum e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

### Example 3: Development Workflow

```bash
# Install local plugin for development
mockforge plugin install ./my-plugin-dev

# Test changes
mockforge serve --admin

# Reinstall after changes
mockforge plugin install ./my-plugin-dev --force
```

### Example 4: Private Repository

```bash
# Configure SSH key
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_rsa

# Install from private GitHub repo
mockforge plugin install git@github.com:company/private-plugin.git#v1.0.0
```

## 🚦 Best Practices

### 1. Always Pin Versions

For production deployments, always specify versions:

```bash
# Good
mockforge plugin install https://github.com/user/plugin#v1.0.0

# Avoid
mockforge plugin install https://github.com/user/plugin
```

### 2. Verify Checksums

For critical plugins, always verify checksums:

```bash
# Download checksum file
curl https://example.com/plugin.zip.sha256 -o checksum.txt

# Install with verification
mockforge plugin install https://example.com/plugin.zip \
  --checksum $(cat checksum.txt)
```

### 3. Use Private Registries

For internal plugins, use private Git repositories or self-hosted registries.

### 4. Regular Updates

Keep plugins up-to-date:

```bash
# Check for updates weekly
mockforge plugin update --all
```

### 5. Test Before Production

Always test plugins in development before using in production:

```bash
# Install in dev environment
mockforge plugin install <source> --force

# Test thoroughly
mockforge serve --config dev-config.yaml

# Then deploy to production
```

## 📖 Related Documentation

- [Plugin Development Guide](./development-guide.md)
- [Plugin Security Model](./security/model.md)
- [Plugin API Reference](./api-reference/core.md)
- [Example Plugins](../../examples/plugins/README.md)

## 🤝 Contributing

Found a bug or have a feature request? Please open an issue on GitHub:
https://github.com/SaaSy-Solutions/mockforge/issues

## 📄 License

MIT OR Apache-2.0
