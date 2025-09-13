# Building from Source

This guide covers building MockForge from source code, including prerequisites, build processes, and troubleshooting common build issues.

## Prerequisites

Before building MockForge, ensure you have the required development tools installed.

### System Requirements

- **Rust**: Version 1.70.0 or later
- **Cargo**: Included with Rust
- **Git**: For cloning the repository
- **C/C++ Compiler**: For native dependencies

### Platform-Specific Requirements

#### Linux (Ubuntu/Debian)

```bash
# Install build essentials
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### macOS

```bash
# Install Xcode command line tools
xcode-select --install

# Install Homebrew (optional, for additional tools)
# /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### Windows

```powershell
# Install Visual Studio Build Tools
# Download from: https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Install Rust
# Download from: https://rustup.rs/
# Or use winget: winget install --id Rustlang.Rustup
```

### Rust Setup Verification

```bash
# Verify Rust installation
rustc --version
cargo --version

# Update to latest stable
rustup update stable
```

## Cloning the Repository

```bash
# Clone the repository
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge

# Initialize submodules (if any)
git submodule update --init --recursive
```

## Build Process

### Basic Build

```bash
# Build all crates in debug mode (default)
cargo build

# Build in release mode for production
cargo build --release

# Build specific crate
cargo build -p mockforge-cli
```

### Build Outputs

After building, binaries are available in:

```bash
# Debug builds
target/debug/mockforge-cli

# Release builds
target/release/mockforge-cli
```

### Build Features

MockForge supports conditional compilation features:

```bash
# Build with all features enabled
cargo build --all-features

# Build with specific features
cargo build --features "grpc,websocket"

# List available features
cargo metadata --format-version 1 | jq '.packages[] | select(.name == "mockforge-cli") | .features'
```

## Development Workflow

### Development Builds

```bash
# Quick development builds
cargo build

# Run tests during development
cargo test

# Run specific tests
cargo test --package mockforge-core --lib
```

### Watch Mode Development

```bash
# Install cargo-watch for automatic rebuilds
cargo install cargo-watch

# Watch for changes and rebuild
cargo watch -x build

# Watch and run tests
cargo watch -x test

# Watch and run specific binary
cargo watch -x "run --bin mockforge-cli -- --help"
```

### IDE Setup

#### VS Code

Install recommended extensions:
- `rust-lang.rust-analyzer`
- `ms-vscode.vscode-json`
- `redhat.vscode-yaml`

#### IntelliJ/CLion

Install Rust plugin through marketplace.

### Debugging

```bash
# Build with debug symbols
cargo build

# Run with debugger
rust-gdb target/debug/mockforge-cli

# Or use lldb on macOS
rust-lldb target/debug/mockforge-cli
```

## Advanced Build Options

### Cross-Compilation

```bash
# Install cross-compilation targets
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-gnu

# Build for different architectures
cargo build --target x86_64-unknown-linux-musl
cargo build --target aarch64-unknown-linux-gnu
```

### Custom Linker

```bash
# Use mold linker for faster linking (Linux)
sudo apt install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
cargo build
```

### Build Caching

```bash
# Use sccache for faster rebuilds
cargo install sccache
export RUSTC_WRAPPER=sccache
cargo build
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests for specific package
cargo test -p mockforge-core

# Run integration tests
cargo test --test integration

# Run with release optimizations
cargo test --release
```

### Test Coverage

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html

# Open coverage report
open tarpaulin-report.html
```

### Benchmarking

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench benchmark_name
```

## Code Quality

### Linting

```bash
# Run clippy lints
cargo clippy

# Run with pedantic mode
cargo clippy -- -W clippy::pedantic

# Auto-fix some issues
cargo clippy --fix
```

### Formatting

```bash
# Check code formatting
cargo fmt --check

# Auto-format code
cargo fmt
```

### Security Auditing

```bash
# Install cargo-audit
cargo install cargo-audit

# Audit dependencies for security vulnerabilities
cargo audit
```

## Documentation

### Building Documentation

```bash
# Build API documentation
cargo doc

# Open documentation in browser
cargo doc --open

# Build documentation with private items
cargo doc --document-private-items

# Build for specific package
cargo doc -p mockforge-core
```

### Building mdBook

```bash
# Install mdbook
cargo install mdbook

# Build the documentation
mdbook build

# Serve documentation locally
mdbook serve
```

## Packaging and Distribution

### Creating Releases

```bash
# Create a release build
cargo build --release

# Strip debug symbols (Linux/macOS)
strip target/release/mockforge-cli

# Create distribution archive
tar -czf mockforge-v0.1.0-x86_64-linux.tar.gz \
  -C target/release mockforge-cli

# Create Debian package
cargo install cargo-deb
cargo deb
```

### Docker Builds

```bash
# Build Docker image
docker build -t mockforge .

# Build with buildkit for faster builds
DOCKER_BUILDKIT=1 docker build -t mockforge .

# Multi-stage build for smaller images
docker build -f Dockerfile.multi-stage -t mockforge .
```

## Troubleshooting Build Issues

### Common Problems

#### Compilation Errors

**Problem**: `error[E0432]: unresolved import`

**Solution**: Check that dependencies are properly specified in `Cargo.toml`

```bash
# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo build
```

#### Linker Errors

**Problem**: `undefined reference to...`

**Solution**: Install system dependencies

```bash
# Ubuntu/Debian
sudo apt install libssl-dev pkg-config

# macOS
brew install openssl pkg-config
```

#### Out of Memory

**Problem**: `fatal error: Killed signal terminated program cc1`

**Solution**: Increase available memory or reduce parallelism

```bash
# Reduce parallel jobs
cargo build --jobs 1

# Or set memory limits
export CARGO_BUILD_JOBS=2
```

#### Slow Builds

**Solutions**:

```bash
# Use incremental compilation
export CARGO_INCREMENTAL=1

# Use faster linker
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Use build cache
cargo install sccache
export RUSTC_WRAPPER=sccache
```

### Platform-Specific Issues

#### Windows

```powershell
# Install Windows SDK if missing
# Download from: https://developer.microsoft.com/en-us/windows/downloads/windows-sdk/

# Use different target for static linking
cargo build --target x86_64-pc-windows-msvc
```

#### macOS

```bash
# Install missing headers
open /Library/Developer/CommandLineTools/Packages/macOS_SDK_headers_for_macOS_10.14.pkg

# Or reinstall command line tools
sudo rm -rf /Library/Developer/CommandLineTools
xcode-select --install
```

#### Linux

```bash
# Install additional development libraries
sudo apt install libclang-dev llvm-dev

# For cross-compilation
sudo apt install gcc-aarch64-linux-gnu
```

### Network Issues

```bash
# Clear cargo cache
cargo clean
rm -rf ~/.cargo/registry/cache
rm -rf ~/.cargo/git/checkouts

# Use different registry
export CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
```

### Dependency Conflicts

```bash
# Update Cargo.lock
cargo update

# Resolve conflicts
cargo update -p package-name

# Use cargo-tree to visualize dependencies
cargo install cargo-tree
cargo tree
```

## Performance Optimization

### Release Builds

```bash
# Optimized release build
cargo build --release

# With Link-Time Optimization (LTO)
export RUSTFLAGS="-C opt-level=3 -C lto=fat -C codegen-units=1"
cargo build --release
```

### Profile-Guided Optimization (PGO)

```bash
# Build with instrumentation
export RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data"
cargo build --release

# Run instrumented binary with representative workload
./target/release/mockforge-cli serve --spec examples/openapi-demo.json &
sleep 10
curl -s http://localhost:3000/users > /dev/null
pkill mockforge-cli

# Build optimized version
export RUSTFLAGS="-Cprofile-use=/tmp/pgo-data"
cargo build --release
```

## Contributing to the Build System

### Adding New Dependencies

```toml
# Add to workspace Cargo.toml
[workspace.dependencies]
new-dependency = "1.0"

# Use in crate Cargo.toml
[dependencies]
new-dependency = { workspace = true }
```

### Adding Build Scripts

```rust
// build.rs
fn main() {
    // Generate code or check dependencies
    println!("cargo:rerun-if-changed=proto/");
    tonic_build::compile_protos("proto/service.proto").unwrap();
}
```

### Custom Build Profiles

```toml
# In Cargo.toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"

[profile.dev]
opt-level = 0
debug = true
overflow-checks = true
```

This comprehensive build guide ensures developers can successfully compile, test, and contribute to MockForge across different platforms and development environments.
