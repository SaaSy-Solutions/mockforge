# MockForge Publishing Guide

This guide explains how to publish MockForge crates to crates.io.

## Prerequisites

1. **Crates.io Account**: Create an account at [crates.io](https://crates.io)
2. **API Token**: Get your API token from [crates.io/me](https://crates.io/me)
3. **Rust Toolchain**: Ensure you have the latest Rust toolchain installed

## Quick Start

### 1. Set up your API token

```bash
export CRATES_IO_TOKEN=your_token_here
```

### 2. Test the publishing process (dry run)

```bash
# Test without actually publishing
./scripts/publish-crates.sh --dry-run
```

### 3. Publish all crates

```bash
# Publish all crates in the correct order
./scripts/publish-crates.sh
```

## Publishing Script Usage

The `scripts/publish-crates.sh` script handles the complex dependency chain automatically.

### Options

- `--dry-run`: Test the process without publishing
- `--convert-only`: Only convert dependencies, don't publish
- `--restore`: Restore path dependencies for development
- `--wait-time SECONDS`: Wait time between publishes (default: 30)
- `--help`: Show help message

### Examples

```bash
# Test the publishing process
./scripts/publish-crates.sh --dry-run

# Only convert dependencies (useful for testing)
./scripts/publish-crates.sh --convert-only

# Restore development dependencies after publishing
./scripts/publish-crates.sh --restore

# Custom wait time between publishes
./scripts/publish-crates.sh --wait-time 60
```

## Publishing Process

The script publishes crates in two phases:

### Phase 1: Base Crates (No Internal Dependencies)
1. `mockforge-core` - Core library
2. `mockforge-data` - Data generation
3. `mockforge-plugin-core` - Plugin system core
4. `mockforge-plugin-sdk` - Plugin development SDK

### Phase 2: Dependent Crates
After converting path dependencies to version dependencies:
1. `mockforge-plugin-loader` - Plugin loading
2. `mockforge-http` - HTTP/REST mocking
3. `mockforge-grpc` - gRPC mocking
4. `mockforge-ws` - WebSocket mocking
5. `mockforge-graphql` - GraphQL mocking
6. `mockforge-mqtt` - MQTT mocking
7. `mockforge-smtp` - SMTP email mocking
8. `mockforge-amqp` - AMQP messaging mocking
9. `mockforge-kafka` - Kafka streaming mocking
10. `mockforge-ftp` - FTP mocking
11. `mockforge-bench` - Benchmarking utilities
12. `mockforge-k8s-operator` - Kubernetes operator
13. `mockforge-registry-server` - Plugin registry server

## Manual Publishing (Alternative)

If you prefer to publish manually:

### 1. Publish base crates first

```bash
cargo publish -p mockforge-core
# Wait 30 seconds
cargo publish -p mockforge-data
# Wait 30 seconds
cargo publish -p mockforge-plugin-core
# Wait 30 seconds
cargo publish -p mockforge-plugin-sdk
```

### 2. Convert dependencies

The script automatically converts:
- `mockforge-core = { path = "../mockforge-core" }` → `mockforge-core = "0.1.0"`
- `mockforge-data = { path = "../mockforge-data" }` → `mockforge-data = "0.1.0"`
- `mockforge-plugin-core = { path = "../mockforge-plugin-core" }` → `mockforge-plugin-core = "0.1.0"`

### 3. Publish dependent crates

```bash
cargo publish -p mockforge-plugin-loader
# Wait 30 seconds
cargo publish -p mockforge-http
# ... continue with remaining crates
```

### 4. Restore development dependencies

```bash
./scripts/publish-crates.sh --restore
```

## Troubleshooting

### Common Issues

1. **"crate not found" errors**
   - Ensure you're publishing in the correct order
   - Wait longer between publishes (increase `--wait-time`)

2. **"already exists" errors**
   - The crate version already exists on crates.io
   - Bump the version in `Cargo.toml` and try again

3. **Authentication errors**
   - Check your `CRATES_IO_TOKEN` is correct
   - Ensure the token has publish permissions

4. **Dependency errors**
   - Run `cargo check --workspace` to verify all dependencies resolve
   - Use `--convert-only` to test dependency conversion

### Recovery

If publishing fails partway through:

1. **Check what was published**:
   ```bash
   # Check published versions
   cargo search mockforge-core
   ```

2. **Resume from where you left off**:
   ```bash
   # Publish remaining crates individually
   cargo publish -p mockforge-http
   ```

3. **Restore development state**:
   ```bash
   ./scripts/publish-crates.sh --restore
   ```

## Post-Publishing

After successful publishing:

1. **Restore development dependencies**:
   ```bash
   ./scripts/publish-crates.sh --restore
   ```

2. **Update documentation**:
   - Update README.md with new version badges
   - Update any hardcoded version references

3. **Test installation**:
   ```bash
   cargo install mockforge-cli
   ```

## Version Management

To publish new versions:

1. **Update version in workspace**:
   ```bash
   # Update version in Cargo.toml workspace section
   # Or use cargo-release for automated versioning
   cargo release patch
   ```

2. **Publish updated crates**:
   ```bash
   ./scripts/publish-crates.sh
   ```

## Security Notes

- Never commit your `CRATES_IO_TOKEN` to version control
- Use environment variables or secure credential storage
- Consider using `cargo login` for interactive authentication

## Support

If you encounter issues:

1. Check the [crates.io documentation](https://doc.rust-lang.org/cargo/reference/publishing.html)
2. Review the [cargo publish documentation](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
3. Check MockForge's [GitHub issues](https://github.com/SaaSy-Solutions/mockforge/issues)
