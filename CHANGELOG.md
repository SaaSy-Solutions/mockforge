# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of MockForge
- HTTP API mocking with OpenAPI support
- gRPC service mocking with Protocol Buffers
- WebSocket connection mocking with replay functionality
- CLI tool for easy local development
- Admin UI for managing mock servers
- Comprehensive documentation with mdBook
- GitHub Actions CI/CD pipeline
- Security audit integration
- Pre-commit hooks for code quality

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A

---

## Release Process

This project uses [cargo-release](https://github.com/crate-ci/cargo-release) for automated releases.

### Creating a Release

1. **Patch Release** (bug fixes):
   ```bash
   make release-patch
   ```

2. **Minor Release** (new features):
   ```bash
   make release-minor
   ```

3. **Major Release** (breaking changes):
   ```bash
   make release-major
   ```

### Manual Release Process

If you need to do a manual release:

1. Update version in `Cargo.toml` files
2. Update `CHANGELOG.md` with release notes
3. Commit changes: `git commit -m "chore: release vX.Y.Z"`
4. Tag: `git tag vX.Y.Z`
5. Push: `git push && git push --tags`
6. Publish to crates.io: `cargo publish`

### Pre-release Checklist

- [ ] All tests pass (`make test`)
- [ ] Code formatted (`make fmt`)
- [ ] Lints pass (`make clippy`)
- [ ] Security audit passes (`make audit`)
- [ ] Documentation updated
- [ ] Changelog updated
- [ ] Version bumped in all `Cargo.toml` files
- [ ] Breaking changes documented (if any)
- [ ] CI passes on all branches
