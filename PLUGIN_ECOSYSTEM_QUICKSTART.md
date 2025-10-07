# MockForge Plugin Ecosystem - Quick Start Guide

Get started with the MockForge Plugin Ecosystem in minutes!

## 🚀 Quick Start

### For Plugin Users

#### Install a Plugin

```bash
# From a URL
mockforge plugin install https://example.com/my-plugin.zip

# From GitHub (with version pinning)
mockforge plugin install https://github.com/user/awesome-auth-plugin#v1.0.0

# From a local file
mockforge plugin install ./my-plugin.zip

# With checksum verification
mockforge plugin install https://url.com/plugin.zip --checksum abc123...
```

#### Manage Installed Plugins

```bash
# List all installed plugins
mockforge plugin list

# Show detailed information
mockforge plugin list --detailed

# Get info about a specific plugin
mockforge plugin info my-plugin-id

# Update a plugin
mockforge plugin update my-plugin-id

# Update all plugins
mockforge plugin update --all

# Uninstall a plugin
mockforge plugin uninstall my-plugin-id
```

#### Cache Management

```bash
# View cache statistics
mockforge plugin cache-stats

# Clear the cache
mockforge plugin clear-cache
```

### For Plugin Developers

#### Create a New Plugin

```bash
# Create an auth plugin
mockforge-plugin new my-auth-plugin --type auth \
  --author "Your Name" \
  --email "you@example.com"

# Create other types
mockforge-plugin new my-template-plugin --type template
mockforge-plugin new my-response-plugin --type response
mockforge-plugin new my-datasource-plugin --type datasource

# Skip git initialization
mockforge-plugin new my-plugin --type auth --no-git
```

#### Build Your Plugin

```bash
cd my-auth-plugin

# Build in debug mode
mockforge-plugin build

# Build in release mode (optimized)
mockforge-plugin build --release
```

The build command will:
- Check if `wasm32-wasi` target is installed
- Install it automatically if needed
- Build your plugin as a WASM module
- Show build progress and errors

#### Test Your Plugin

```bash
# Run all tests
mockforge-plugin test

# Run specific test
mockforge-plugin test --test test_authentication
```

#### Validate Your Plugin

```bash
# Validate manifest and structure
mockforge-plugin validate
```

This checks:
- ✅ `plugin.yaml` exists and is valid
- ✅ Required fields are present
- ✅ `Cargo.toml` is configured correctly
- ✅ `src/lib.rs` exists
- ✅ Plugin type is specified

#### Package for Distribution

```bash
# Create a distributable ZIP archive
mockforge-plugin package
```

This creates:
- ✅ `my-auth-plugin.zip` with manifest + WASM
- ✅ SHA-256 checksum for verification
- ✅ Ready to share or publish

#### View Plugin Information

```bash
# Show plugin details
mockforge-plugin info
```

Displays:
- Plugin ID, version, name
- Author information
- Capabilities and resource limits
- Build status (debug/release)
- File locations

#### Clean Build Artifacts

```bash
# Clean all build artifacts
mockforge-plugin clean
```

## 📚 Complete Workflow Examples

### Example 1: Create and Distribute an Auth Plugin

```bash
# 1. Create the project
mockforge-plugin new my-auth --type auth --author "Me" --email "me@example.com"
cd my-auth

# 2. Implement your logic
# Edit src/lib.rs

# 3. Test it
mockforge-plugin test

# 4. Build release version
mockforge-plugin build --release

# 5. Validate everything is correct
mockforge-plugin validate

# 6. Package for distribution
mockforge-plugin package

# Output:
# ✅ Plugin packaged: my-auth.zip
# ✅ SHA-256: abc123...

# 7. Distribute
# - Upload to GitHub releases
# - Share the .zip file
# - Publish to registry (coming in Phase 3)
```

### Example 2: Install and Use a Plugin

```bash
# 1. Find a plugin (e.g., on GitHub)
# https://github.com/mockforge/plugin-saml-auth

# 2. Install it
mockforge plugin install https://github.com/mockforge/plugin-saml-auth#v2.0.0

# 3. Verify installation
mockforge plugin list

# 4. Check plugin details
mockforge plugin info saml-auth

# 5. Use it in your MockForge configuration
# (configure in your mockforge.yaml)

# 6. Update when new version available
mockforge plugin update saml-auth
```

### Example 3: Development with Iterations

```bash
# Create plugin
mockforge-plugin new my-template --type template
cd my-template

# Develop with test-driven approach
while true; do
  # Edit code
  nano src/lib.rs

  # Run tests
  mockforge-plugin test

  # If tests pass, break
  if [ $? -eq 0 ]; then break; fi
done

# Build and package
mockforge-plugin build --release
mockforge-plugin package

# Share with team
git add .
git commit -m "Release v1.0.0"
git tag v1.0.0
git push origin main --tags

# Team members can now install
# mockforge plugin install git@github.com:yourteam/my-template#v1.0.0
```

## 🎯 Plugin Types

### 1. Auth Plugins

**Purpose**: Custom authentication methods (SAML, LDAP, OAuth, etc.)

```bash
mockforge-plugin new my-auth --type auth
```

**Use Cases**:
- Enterprise SSO integration
- Custom authentication protocols
- Multi-factor authentication
- Token validation

### 2. Template Plugins

**Purpose**: Custom template rendering engines

```bash
mockforge-plugin new my-template --type template
```

**Use Cases**:
- Specialized template syntax
- Integration with existing template systems
- Advanced text processing
- Custom data transformation

### 3. Response Plugins

**Purpose**: HTTP response modification

```bash
mockforge-plugin new my-response --type response
```

**Use Cases**:
- Header injection
- Response transformation
- Content filtering
- Protocol conversion

### 4. DataSource Plugins

**Purpose**: External data integration

```bash
mockforge-plugin new my-datasource --type datasource
```

**Use Cases**:
- Database connections
- API integrations
- File system access
- Real-time data streams

## 🔒 Security Best Practices

### Plugin Installation:

```bash
# ✅ ALWAYS verify checksums for untrusted sources
mockforge plugin install https://example.com/plugin.zip \
  --checksum <expected-sha256>

# ✅ Pin versions in production
mockforge plugin install github:user/plugin#v1.2.3  # Not #main

# ✅ Validate before installing
mockforge plugin validate https://example.com/plugin.zip
```

### Plugin Development:

```yaml
# ✅ Request minimal capabilities
capabilities:
  network: false      # Only if needed
  filesystem: false   # Only if needed

# ✅ Set conservative resource limits
resource_limits:
  max_memory_bytes: 10485760  # 10MB
  max_cpu_time_ms: 5000       # 5 seconds
```

## 🐛 Troubleshooting

### Build Issues

**Problem**: `wasm32-wasi target not found`

```bash
# Solution: Install manually
rustup target add wasm32-wasi

# Or let the CLI install it
mockforge-plugin build  # Auto-installs if missing
```

**Problem**: `plugin.yaml not found`

```bash
# Solution: Initialize manifest
mockforge-plugin init --type auth
```

### Installation Issues

**Problem**: `Checksum verification failed`

```bash
# Solution: Get correct checksum or install without verification
mockforge plugin install https://url.com/plugin.zip  # No checksum
```

**Problem**: `Git repository not found`

```bash
# Solution: Check URL and ensure git is installed
git --version
```

### Runtime Issues

**Problem**: Plugin not appearing in list

```bash
# Solution: Check installation directory and re-install
mockforge plugin list
mockforge plugin install ./plugin.zip --force
```

## 💡 Tips & Tricks

### Fast Development Cycle

```bash
# Use a shell alias for quick test-build-package
alias plugin-release='mockforge-plugin test && mockforge-plugin build --release && mockforge-plugin package'

# Then just run:
plugin-release
```

### Git-Based Versioning

```bash
# Create plugins/versions/branches
git tag v1.0.0
git push --tags

# Users install specific versions
mockforge plugin install github:you/plugin#v1.0.0
```

### Local Testing

```bash
# Build and install locally
mockforge-plugin build --release
mockforge-plugin package
mockforge plugin install ./my-plugin.zip
```

### Automation

```yaml
# .github/workflows/release.yml
name: Release Plugin
on:
  push:
    tags:
      - 'v*'
jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-wasi
      - name: Build and Package
        run: |
          cargo install mockforge-plugin-cli
          mockforge-plugin build --release
          mockforge-plugin package
      - name: Upload Release
        uses: actions/upload-release-asset@v1
        with:
          upload_url: ${{ github.event.release.upload_url }}
          asset_path: ./my-plugin.zip
```

## 📖 Next Steps

- 📚 Read the full documentation: `docs/plugins/`
- 🔌 Browse example plugins: `examples/plugins/`
- 🛠️ Explore the SDK: `crates/mockforge-plugin-sdk/`
- 📦 Check the registry (Phase 3): Coming soon!

## 🆘 Getting Help

- 📝 File issues: https://github.com/SaaSy-Solutions/mockforge/issues
- 💬 Community discussions: (Coming soon)
- 📖 Documentation: `docs/plugins/remote-loading.md`

---

**Happy Plugin Development!** 🎉
