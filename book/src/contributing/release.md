# Release Process

This guide outlines the complete process for releasing new versions of MockForge, from planning through deployment and post-release activities.

## Release Planning

### Version Numbering

MockForge follows [Semantic Versioning](https://semver.org/) (SemVer):

```
MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]

Examples:
- 1.0.0 (stable release)
- 1.1.0 (minor release with new features)
- 1.1.1 (patch release with bug fixes)
- 2.0.0-alpha.1 (pre-release)
- 1.0.0+20230912 (build metadata)
```

#### When to Increment

- **MAJOR** (X.0.0): Breaking changes to public API
- **MINOR** (X.Y.0): New features, backward compatible
- **PATCH** (X.Y.Z): Bug fixes, backward compatible

### Release Types

#### Major Releases
- Breaking API changes
- Major feature additions
- Architectural changes
- Extended testing period (2-4 weeks beta)

#### Minor Releases
- New features and enhancements
- Backward compatible API changes
- Standard testing period (1-2 weeks)

#### Patch Releases
- Critical bug fixes
- Security patches
- Documentation updates
- Minimal testing period (3-5 days)

#### Pre-releases
- Alpha/Beta/RC versions
- Feature previews
- Breaking change previews
- Limited distribution

## Pre-Release Checklist

### 1. Code Quality Verification

```bash
# Run complete test suite
make test

# Run integration tests
make test-integration

# Run E2E tests
make test-e2e

# Check code quality
make lint
make format-check

# Security audit
cargo audit

# Check for unused dependencies
cargo +nightly udeps

# Performance benchmarks
make benchmark
```

### 2. Documentation Updates

```bash
# Update CHANGELOG.md with release notes
# Update version numbers in documentation
# Build and test documentation
make docs
make docs-serve

# Test documentation links
mdbook test
```

### 3. Version Bump

```bash
# Update version in Cargo.toml files
# Update version in package metadata
# Update version in documentation

# Example version bump script
#!/bin/bash
NEW_VERSION=$1

# Update workspace Cargo.toml
sed -i "s/^version = .*/version = \"$NEW_VERSION\"/" Cargo.toml

# Update all crate Cargo.toml files
find crates -name "Cargo.toml" -exec sed -i "s/^version = .*/version = \"$NEW_VERSION\"/" {} \;

# Update README and documentation version references
sed -i "s/mockforge [0-9]\+\.[0-9]\+\.[0-9]\+/mockforge $NEW_VERSION/g" README.md
```

### 4. Branch Management

```bash
# Create release branch
git checkout -b release/v$NEW_VERSION

# Cherry-pick approved commits
# Or merge from develop/main

# Tag the release
git tag -a v$NEW_VERSION -m "Release version $NEW_VERSION"

# Push branch and tag
git push origin release/v$NEW_VERSION
git push origin v$NEW_VERSION
```

## Release Build Process

### 1. Build Verification

```bash
# Clean build
cargo clean

# Build all targets
cargo build --release --all-targets

# Build specific platforms if needed
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-apple-darwin
cargo build --release --target x86_64-pc-windows-msvc

# Test release build
./target/release/mockforge-cli --version
```

### 2. Binary Distribution

#### Linux/macOS Packages

```bash
# Strip debug symbols
strip target/release/mockforge-cli

# Create distribution archives
VERSION=1.0.0
tar -czf mockforge-v${VERSION}-x86_64-linux.tar.gz \
  -C target/release mockforge-cli

tar -czf mockforge-v${VERSION}-x86_64-macos.tar.gz \
  -C target/release mockforge-cli
```

#### Debian Packages

```bash
# Install cargo-deb
cargo install cargo-deb

# Build .deb package
cargo deb

# Test package installation
sudo dpkg -i target/debian/mockforge_*.deb
```

#### Docker Images

```dockerfile
# Dockerfile.release
FROM rust:1.70-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mockforge-cli /usr/local/bin/mockforge-cli
EXPOSE 3000 3001 50051 9080
CMD ["mockforge-cli", "serve"]
```

```bash
# Build and push Docker image
docker build -f Dockerfile.release -t mockforge:$VERSION .
docker tag mockforge:$VERSION mockforge:latest
docker push mockforge:$VERSION
docker push mockforge:latest
```

### 3. Cross-Platform Builds

```bash
# Use cross for cross-compilation
cargo install cross

# Build for different architectures
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target x86_64-unknown-linux-musl

# Create release archives for each platform
for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-apple-darwin x86_64-pc-windows-msvc; do
  cross build --release --target $target
  if [[ $target == *"windows"* ]]; then
    zip -j mockforge-$VERSION-$target.zip target/$target/release/mockforge-cli.exe
  else
    tar -czf mockforge-$VERSION-$target.tar.gz -C target/$target/release mockforge-cli
  fi
done
```

## Release Deployment

### 1. GitHub Release

```bash
# Create GitHub release (manual or automated)
gh release create v$VERSION \
  --title "MockForge v$VERSION" \
  --notes-file release-notes.md \
  --draft

# Upload release assets
gh release upload v$VERSION \
  mockforge-v$VERSION-x86_64-linux.tar.gz \
  mockforge-v$VERSION-x86_64-macos.tar.gz \
  mockforge-v$VERSION-x86_64-windows.zip \
  mockforge_$VERSION_amd64.deb

# Publish release
gh release edit v$VERSION --draft=false
```

### 2. Package Registries

#### Crates.io Publication

```bash
# Publish all crates to crates.io
# Note: Must be done in dependency order

# Publish core first
cd crates/mockforge-core
cargo publish

# Then other crates
cd ../mockforge-http
cargo publish

cd ../mockforge-ws
cargo publish

cd ../mockforge-grpc
cargo publish

cd ../mockforge-data
cargo publish

cd ../mockforge-ui
cargo publish

# Finally CLI
cd ../mockforge-cli
cargo publish
```

#### Docker Hub

> **Note**: Docker Hub publishing is planned for future releases. The organization and repository need to be set up first.

Once Docker Hub is configured, use these commands:

```bash
# Build the Docker image with version tag
docker build -t saasy-solutions/mockforge:$VERSION .
docker tag saasy-solutions/mockforge:$VERSION saasy-solutions/mockforge:latest

# Push to Docker Hub (requires authentication)
docker login
docker push saasy-solutions/mockforge:$VERSION
docker push saasy-solutions/mockforge:latest
```

For now, users should build the Docker image locally as documented in the [Installation Guide](../getting-started/installation.md#method-2-docker-containerized).

### 3. Homebrew (macOS)

```ruby
# Formula/mockforge.rb
class Mockforge < Formula
  desc "Advanced API Mocking Platform"
  homepage "https://github.com/SaaSy-Solutions/mockforge"
  url "https://github.com/SaaSy-Solutions/mockforge/releases/download/v#{version}/mockforge-v#{version}-x86_64-macos.tar.gz"
  sha256 "..."

  def install
    bin.install "mockforge-cli"
  end

  test do
    system "#{bin}/mockforge-cli", "--version"
  end
end
```

### 4. Package Managers

#### APT Repository (Ubuntu/Debian)

```bash
# Set up PPA or repository
# Upload .deb packages
# Update package indices
```

#### Snapcraft

```yaml
# snapcraft.yaml
name: mockforge
version: '1.0.0'
summary: Advanced API Mocking Platform
description: |
  MockForge is a comprehensive API mocking platform supporting HTTP, WebSocket, and gRPC protocols.

grade: stable
confinement: strict

apps:
  mockforge:
    command: mockforge-cli
    plugs: [network, network-bind]

parts:
  mockforge:
    plugin: rust
    source: .
    build-packages: [pkg-config, libssl-dev]
```

## Post-Release Activities

### 1. Announcement

#### GitHub Release Notes

```markdown
## What's New in MockForge v1.0.0

### 🚀 Major Features
- Multi-protocol support (HTTP, WebSocket, gRPC)
- Advanced templating system
- Web-based admin UI
- Comprehensive testing framework

### 🐛 Bug Fixes
- Fixed template rendering performance
- Resolved WebSocket connection stability
- Improved error messages

### 📚 Documentation
- Complete API reference
- Getting started guides
- Troubleshooting documentation

### 🤝 Contributors
Special thanks to all contributors!

### 🔗 Links
- [Documentation](https://docs.mockforge.dev)
- [GitHub Repository](https://github.com/SaaSy-Solutions/mockforge)
- [Issue Tracker](https://github.com/SaaSy-Solutions/mockforge/issues)
```

#### Social Media & Community

```bash
# Post to social media
# Update Discord/Slack channels
# Send email newsletter
# Update website/blog
```

### 2. Monitoring & Support

#### Release Health Checks

```bash
# Monitor installation success
# Check for immediate bug reports
# Monitor CI/CD pipelines
# Track adoption metrics

# Example monitoring script
#!/bin/bash
VERSION=$1

# Check GitHub release downloads
gh release view v$VERSION --json assets -q '.assets[].downloadCount'

# Check crates.io download stats
curl -s "https://crates.io/api/v1/crates/mockforge-cli/downloads" | jq '.versions[0].downloads'

# Monitor error reports
gh issue list --label bug --state open --limit 10
```

#### Support Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and support
- **Discord/Slack**: Real-time community support
- **Documentation**: Self-service troubleshooting

### 3. Follow-up Releases

#### Hotfix Process

For critical issues discovered post-release:

```bash
# Create hotfix branch from release tag
git checkout -b hotfix/critical-bug-fix v1.0.0

# Apply fix
# Write test
# Update CHANGELOG

# Create patch release
NEW_VERSION=1.0.1
git tag -a v$NEW_VERSION
git push origin v$NEW_VERSION

# Deploy hotfix
```

### 4. Analytics & Metrics

#### Release Metrics

- Download counts across platforms
- Installation success rates
- User adoption and usage patterns
- Performance benchmarks vs previous versions
- Community feedback and sentiment

#### Continuous Improvement

```yaml
# Post-release retrospective template
## Release Summary
- Version: v1.0.0
- Release Date: YYYY-MM-DD
- Duration: X weeks

## What Went Well
- [ ] Smooth release process
- [ ] No critical bugs found
- [ ] Good community reception

## Areas for Improvement
- [ ] Documentation could be clearer
- [ ] Testing took longer than expected
- [ ] More platform support needed

## Action Items
- [ ] Improve release documentation
- [ ] Automate more of the process
- [ ] Add more platform builds
```

## Release Automation

### GitHub Actions Release Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Set version
      run: echo "VERSION=${GITHUB_REF#refs/tags/v}" >> $GITHUB_ENV

    - name: Build release binaries
      run: |
        cargo build --release
        strip target/release/mockforge-cli

    - name: Create release archives
      run: |
        tar -czf mockforge-${VERSION}-linux-x64.tar.gz -C target/release mockforge-cli
        zip mockforge-${VERSION}-linux-x64.zip target/release/mockforge-cli

    - name: Create GitHub release
      uses: actions/create-release@v1
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        tag_name: ${{ github.ref }}
        release_name: MockForge ${{ env.VERSION }}
        body: |
          ## What's New

          See [CHANGELOG.md](CHANGELOG.md) for details.

          ## Downloads

          - Linux x64: [mockforge-${{ env.VERSION }}-linux-x64.tar.gz](mockforge-${{ env.VERSION }}-linux-x64.tar.gz)
        draft: false
        prerelease: false

    - name: Upload release assets
      uses: actions/upload-release-asset@v1
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        upload_url: ${{ steps.create_release.outputs.upload_url }}
        asset_path: ./mockforge-${{ env.VERSION }}-linux-x64.tar.gz
        asset_name: mockforge-${{ env.VERSION }}-linux-x64.tar.gz
        asset_content_type: application/gzip
```

### Automated Publishing

```yaml
# Publish to crates.io on release
- name: Publish to crates.io
  run: cargo publish --token ${{ secrets.CRATES_IO_TOKEN }}
  if: startsWith(github.ref, 'refs/tags/')

# Build and push Docker image
- name: Build and push Docker image
  uses: docker/build-push-action@v3
  with:
    context: .
    push: true
    tags: mockforge/mockforge:${{ env.VERSION }},mockforge/mockforge:latest
```

## Emergency Releases

### Security Vulnerabilities

For security issues requiring immediate release:

1. **Assess Severity**: Determine CVSS score and impact
2. **Develop Fix**: Create minimal fix with comprehensive tests
3. **Bypass Normal Process**: Skip extended testing for critical security fixes
4. **Accelerated Release**: 24-48 hour release cycle
5. **Public Disclosure**: Coordinate with security community

### Critical Bug Fixes

For show-stopping bugs affecting production:

1. **Immediate Assessment**: Evaluate user impact and severity
2. **Rapid Development**: 1-2 day fix development
3. **Limited Testing**: Focus on regression and critical path tests
4. **Fast-Track Release**: 3-5 day release cycle

This comprehensive release process ensures MockForge releases are reliable, well-tested, and properly distributed across all supported platforms and package managers.
