# Development Setup

This guide helps contributors get started with MockForge development, including environment setup, development workflow, and project structure.

## Prerequisites

Before contributing to MockForge, ensure you have the following installed:

### Required Tools

- **Rust**: Version 1.70.0 or later
- **Cargo**: Included with Rust
- **Git**: For version control
- **C/C++ Compiler**: For native dependencies
- **Docker**: For containerized development and testing

### Recommended Tools

- **Visual Studio Code** or **IntelliJ/CLion** with Rust plugins
- **cargo-watch** for automatic rebuilds
- **cargo-edit** for dependency management
- **cargo-audit** for security scanning
- **mdbook** for documentation development

## Environment Setup

### 1. Install Rust

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add Cargo to PATH
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### 2. Clone the Repository

```bash
# Clone with SSH (recommended for contributors)
git clone git@github.com:SaaSy-Solutions/mockforge.git

# Or with HTTPS
git clone https://github.com/SaaSy-Solutions/mockforge.git

cd mockforge

# Initialize submodules if any
git submodule update --init --recursive
```

### 3. Install Development Tools

```bash
# Install cargo-watch for automatic rebuilds
cargo install cargo-watch

# Install cargo-edit for dependency management
cargo install cargo-edit

# Install cargo-audit for security scanning
cargo install cargo-audit

# Install mdbook for documentation
cargo install mdbook mdbook-linkcheck mdbook-toc

# Install additional development tools
cargo install cargo-tarpaulin cargo-udeps cargo-outdated
```

### 4. Verify Setup

```bash
# Build the project
cargo build

# Run tests
cargo test

# Check code quality
cargo clippy
cargo fmt --check
```

## Development Workflow

### Daily Development

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make changes** with frequent testing:
   ```bash
   # Run tests automatically on changes
   cargo watch -x test

   # Or build automatically
   cargo watch -x build
   ```

3. **Follow code quality standards**:
   ```bash
   # Format code
   cargo fmt

   # Lint code
   cargo clippy -- -W clippy::pedantic

   # Run security audit
   cargo audit
   ```

4. **Write tests** for new functionality:
   ```bash
   # Add unit tests
   cargo test --lib

   # Add integration tests
   cargo test --test integration
   ```

### IDE Configuration

#### Visual Studio Code

1. Install extensions:
   - `rust-lang.rust-analyzer` - Rust language support
   - `ms-vscode.vscode-json` - JSON support
   - `redhat.vscode-yaml` - YAML support
   - `ms-vscode.vscode-docker` - Docker support

2. Recommended settings in `.vscode/settings.json`:
   ```json
   {
     "rust-analyzer.checkOnSave.command": "clippy",
     "rust-analyzer.cargo.allFeatures": true,
     "editor.formatOnSave": true,
     "editor.codeActionsOnSave": {
       "source.fixAll": "explicit"
     }
   }
   ```

#### IntelliJ/CLion

1. Install Rust plugin from marketplace
2. Enable external linter (clippy)
3. Configure code style to match project standards

### Pre-commit Setup

Install pre-commit hooks to ensure code quality:

```bash
# Install pre-commit if not already installed
pip install pre-commit

# Install hooks
pre-commit install

# Run on all files
pre-commit run --all-files
```

## Project Structure

```
mockforge/
├── crates/                    # Rust crates
│   ├── mockforge-cli/        # Command-line interface
│   ├── mockforge-core/       # Shared core functionality
│   ├── mockforge-http/       # HTTP REST API mocking
│   ├── mockforge-ws/         # WebSocket connection mocking
│   ├── mockforge-grpc/       # gRPC service mocking
│   ├── mockforge-data/       # Synthetic data generation
│   └── mockforge-ui/         # Web-based admin interface
├── docs/                     # Technical documentation
├── examples/                 # Usage examples
├── book/                     # User documentation (mdBook)
│   └── src/
├── fixtures/                 # Test fixtures
├── scripts/                  # Development scripts
├── tools/                    # Development tools
├── Cargo.toml               # Workspace configuration
├── Cargo.lock               # Dependency lock file
├── Makefile                # Development tasks
├── docker-compose.yml      # Development environment
└── README.md               # Project overview
```

## Development Tasks

### Common Make Targets

```bash
# Build all crates
make build

# Run tests
make test

# Run integration tests
make test-integration

# Build documentation
make docs

# Serve documentation locally
make docs-serve

# Run linter
make lint

# Format code
make format

# Clean build artifacts
make clean
```

### Custom Development Scripts

Several development scripts are available in the `scripts/` directory:

```bash
# Update dependencies
./scripts/update-deps.sh

# Generate API documentation
./scripts/gen-docs.sh

# Run performance benchmarks
./scripts/benchmark.sh

# Check for unused dependencies
./scripts/check-deps.sh
```

## Testing Strategy

### Unit Tests

```bash
# Run unit tests for all crates
cargo test --lib

# Run unit tests for specific crate
cargo test -p mockforge-core

# Run with coverage
cargo tarpaulin --out Html
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration

# Run with verbose output
cargo test --test integration -- --nocapture
```

### End-to-End Tests

```bash
# Run E2E tests (requires Docker)
make test-e2e

# Or run manually
./scripts/test-e2e.sh
```

## Docker Development

### Development Container

```bash
# Build development container
docker build -f Dockerfile.dev -t mockforge-dev .

# Run development environment
docker run -it --rm \
  -v $(pwd):/app \
  -p 3000:3000 \
  -p 3001:3001 \
  -p 50051:50051 \
  -p 9080:9080 \
  mockforge-dev
```

### Testing with Docker

```bash
# Run tests in container
docker run --rm -v $(pwd):/app mockforge-dev cargo test

# Build release binaries
docker run --rm -v $(pwd):/app mockforge-dev cargo build --release
```

## Contributing Workflow

### 1. Choose an Issue

- Check [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues) for open tasks
- Look for issues labeled `good first issue` or `help wanted`
- Comment on the issue to indicate you're working on it

### 2. Create a Branch

```bash
# Create feature branch
git checkout -b feature/issue-number-description

# Or create bugfix branch
git checkout -b bugfix/issue-number-description
```

### 3. Make Changes

- Write clear, focused commits
- Follow the [code style guide](style.md)
- Add tests for new functionality
- Update documentation as needed

### 4. Test Your Changes

```bash
# Run full test suite
make test

# Run integration tests
make test-integration

# Test manually if applicable
cargo run -- serve --spec examples/openapi-demo.json
```

### 5. Update Documentation

```bash
# Update user-facing docs if needed
mdbook build

# Update API docs
cargo doc

# Test documentation links
mdbook test
```

### 6. Submit a Pull Request

```bash
# Ensure branch is up to date
git fetch origin
git rebase origin/main

# Push your branch
git push origin feature/your-feature

# Create PR on GitHub with:
# - Clear title and description
# - Reference to issue number
# - Screenshots/videos for UI changes
# - Test results
```

## Getting Help

### Communication Channels

- **GitHub Issues**: For bugs, features, and general discussion
- **GitHub Discussions**: For questions and longer-form discussion
- **Discord/Slack**: For real-time chat (if available)

### When to Ask for Help

- Stuck on a technical problem for more than 2 hours
- Unsure about design decisions
- Need clarification on requirements
- Found a potential security issue

### Code Review Process

- All PRs require review from at least one maintainer
- CI must pass all checks
- Code coverage should not decrease significantly
- Documentation must be updated for user-facing changes

This setup guide ensures you have everything needed to contribute effectively to MockForge. Happy coding! 🚀
