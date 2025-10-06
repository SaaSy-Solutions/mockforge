# Installation

MockForge can be installed through multiple methods depending on your needs and environment. Choose the installation method that best fits your workflow.

## Prerequisites

Before installing MockForge, ensure you have one of the following:

- **Rust toolchain** (for cargo installation or building from source)
- **Docker** (for containerized deployment)
- **Pre-built binaries** (when available)

## Method 1: Cargo Install (Recommended)

The easiest way to install MockForge is through Cargo, Rust's package manager:

```bash
cargo install mockforge-cli
```

This installs the MockForge CLI globally on your system. After installation, you can verify it's working:

```bash
mockforge --version
```

### Updating

To update to the latest version:

```bash
cargo install mockforge-cli --force
```

## Method 2: Docker (Containerized)

MockForge is also available as a Docker image, which is ideal for:
- Isolated environments
- CI/CD pipelines
- Systems without Rust installed

### Build Docker image

Since pre-built images are not yet published to Docker Hub, build the image locally:

```bash
# Clone and build
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge
docker build -t mockforge .
```

### Run with basic configuration

```bash
docker run -p 3000:3000 -p 3001:3001 -p 50051:50051 -p 9080:9080 \
  -e MOCKFORGE_ADMIN_ENABLED=true \
  -e MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=true \
  mockforge
```

### Alternative: Docker Compose

For a complete setup with all services:

```bash
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge
docker-compose up
```

### Build from source (without Docker)

```bash
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge
docker build -t mockforge .
```

## Method 3: Building from Source

For development or custom builds, you can build MockForge from source:

```bash
git clone https://github.com/SaaSy-Solutions/mockforge.git
cd mockforge
cargo build --release
```

The binary will be available at `target/release/mockforge`.

To install it system-wide after building:

```bash
cargo install --path crates/mockforge-cli
```

## Verification

After installation, verify MockForge is working:

```bash
# Check version
mockforge --version

# View help
mockforge --help

# Start with example configuration
mockforge serve --spec examples/openapi-demo.json --http-port 3000
```

## Platform Support

MockForge supports:
- **Linux** (x86_64, aarch64)
- **macOS** (x86_64, aarch64)
- **Windows** (x86_64)
- **Docker** (any platform with Docker support)

## Troubleshooting Installation

### Cargo installation fails

If `cargo install` fails, ensure you have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Docker permission issues

If Docker commands fail with permission errors:

```bash
# Add user to docker group (Linux)
sudo usermod -aG docker $USER
# Log out and back in for changes to take effect
```

### Port conflicts

If default ports (3000, 3001, 9080, 50051) are in use:

```bash
# Check what's using the ports
lsof -i :3000
lsof -i :3001

# Kill conflicting processes or use different ports
mockforge serve --http-port 3001 --ws-port 3002 --admin-port 8081
```

## Next Steps

Once installed, proceed to the [Quick Start](quick-start.md) guide to create your first mock server, or read about [Basic Concepts](concepts.md) to understand how MockForge works.
