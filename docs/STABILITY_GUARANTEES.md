# MockForge Stability Guarantees

**Version**: 1.0.0
**Effective Date**: 2025-01-27

This document outlines MockForge's stability guarantees and versioning policy for the 1.0 release and beyond.

## Semantic Versioning

MockForge follows [Semantic Versioning](https://semver.org/) (SemVer):

```
MAJOR.MINOR.PATCH

Examples:
- 1.0.0 (initial stable release)
- 1.1.0 (minor release with new features)
- 1.1.1 (patch release with bug fixes)
- 2.0.0 (major release with breaking changes)
```

## Stability Tiers

### Tier 1: Stable Public APIs (1.0+)

**Crates**:
- `mockforge-core` - Core types and utilities
- `mockforge-plugin-core` - Plugin system core
- `mockforge-plugin-sdk` - Plugin development SDK

**Guarantees**:
- ✅ **API Stability**: Public APIs will not change in breaking ways within the same major version
- ✅ **Backward Compatibility**: Code written for 1.0.x will work with 1.y.z (same major version)
- ✅ **Deprecation Policy**: Breaking changes require at least one minor version with deprecation warnings
- ✅ **Documentation**: All public APIs are fully documented

**What This Means**:
- You can safely depend on these crates in production
- Upgrading within the same major version (1.0 → 1.1 → 1.2) will not break your code
- Breaking changes only occur in major version bumps (1.x → 2.0)

### Tier 2: Protocol Implementations (1.0+)

**Crates**:
- `mockforge-http` - HTTP/REST support
- `mockforge-ws` - WebSocket support
- `mockforge-grpc` - gRPC support
- `mockforge-graphql` - GraphQL support
- `mockforge-data` - Data generation

**Guarantees**:
- ✅ **Core Functionality**: Core protocol support is stable
- ✅ **Configuration**: Configuration APIs are stable
- ✅ **Backward Compatibility**: Breaking changes follow deprecation policy
- ⚠️ **Advanced Features**: Some advanced features may evolve

**What This Means**:
- Basic protocol support is stable and production-ready
- Advanced features may be enhanced in minor versions
- Configuration changes are backwards compatible within major versions

### Tier 3: Internal/Experimental (No Stability Guarantee)

**Crates**:
- `mockforge-cli` - CLI binary (internal)
- `mockforge-ui` - Admin UI (internal)
- `mockforge-recorder` - Recording/replay (internal)
- `mockforge-observability` - Observability (internal)
- `mockforge-chaos` - Chaos engineering (experimental)
- `mockforge-reporting` - Reporting (internal)

**Guarantees**:
- ⚠️ **No Stability Guarantee**: These crates may change without notice
- ⚠️ **Not Published**: Most are marked `publish = false`
- ⚠️ **Internal Use**: Intended for internal use only

**What This Means**:
- Do not depend on these crates directly
- Use the CLI binary or public APIs instead
- Internal APIs may change at any time

## Breaking Changes Policy

### Major Versions (X.0.0)

Breaking changes are allowed in major versions:
- API signature changes
- Removal of deprecated APIs
- Configuration format changes
- Behavioral changes

**Process**:
1. Deprecation warnings in previous major version
2. Clear migration guide
3. Extended beta period (if significant)
4. Comprehensive documentation

### Minor Versions (X.Y.0)

Breaking changes are **not allowed** in minor versions:
- ✅ New features may be added
- ✅ Bug fixes may be included
- ✅ Performance improvements
- ❌ No breaking API changes
- ❌ No removal of public APIs

### Patch Versions (X.Y.Z)

Only bug fixes and security patches:
- ✅ Bug fixes
- ✅ Security patches
- ✅ Documentation updates
- ❌ No new features
- ❌ No breaking changes

## Deprecation Policy

### Deprecation Process

1. **Announcement**: Feature marked as deprecated in release notes
2. **Warning Period**: At least one minor version with deprecation warnings
3. **Documentation**: Migration guide provided
4. **Removal**: Removed in next major version

### Example

```
1.0.0: Feature X introduced
1.1.0: Feature X deprecated, feature Y introduced as replacement
1.2.0: Feature X still available but deprecated
1.3.0: Feature X still available but deprecated
2.0.0: Feature X removed, must use feature Y
```

## Supported Versions

### Current Stable

- **1.0.x**: Current stable release
- **Latest Patch**: Always recommended

### Security Support

- **Current Major Version**: Security patches for 6 months after next major release
- **Previous Major Version**: Security patches for 3 months after next major release

### Example Timeline

```
1.0.0 released: Jan 2025
1.1.0 released: Mar 2025
2.0.0 released: Jul 2025
1.0.x security support: Until Jan 2026 (6 months after 2.0.0)
```

## Platform Support

### Supported Platforms

- **Linux**: x86_64, aarch64 (glibc and musl)
- **macOS**: x86_64, Apple Silicon (aarch64)
- **Windows**: x86_64 (MSVC)

### Minimum Requirements

- **Rust**: 1.70+ (stable)
- **Linux**: glibc 2.17+ or musl 1.1.20+
- **macOS**: 10.13+
- **Windows**: Windows 10+

### Platform Support Guarantees

- ✅ **Stable Platforms**: Support maintained for stable releases
- ⚠️ **Beta Platforms**: Experimental support, may have issues
- ❌ **Unsupported Platforms**: No guarantee of functionality

## API Compatibility

### Public API Definition

A public API includes:
- Public structs, enums, traits, and functions
- Configuration file formats
- CLI command-line interfaces
- Protocol wire formats

### Internal API Definition

An internal API includes:
- Private functions and types
- Internal implementation details
- Unstable features behind feature flags
- Experimental APIs

### Compatibility Guarantees

| API Type | Stability | Breaking Changes |
|----------|-----------|------------------|
| Public Stable | ✅ Stable | Only in major versions |
| Public Experimental | ⚠️ May change | May change in minor versions |
| Internal | ❌ No guarantee | May change at any time |

## Feature Flags

### Stable Features

Features without feature flags are stable:
- ✅ Part of public API
- ✅ Follow stability guarantees
- ✅ Safe for production use

### Experimental Features

Features behind feature flags may change:
- ⚠️ May change in minor versions
- ⚠️ May be removed
- ⚠️ Use with caution

## CLI Compatibility

### Command-Line Interface

- ✅ **Stable Commands**: Core commands stable within major version
- ✅ **Backward Compatible**: Old commands continue to work
- ⚠️ **New Options**: May be added in minor versions
- ❌ **Breaking Changes**: Only in major versions

### Configuration Files

- ✅ **Format Stability**: Configuration format stable within major version
- ✅ **Backward Compatible**: Old configs work with new versions
- ⚠️ **New Options**: May be added in minor versions
- ❌ **Breaking Changes**: Only in major versions with migration guide

## Migration Support

### Migration Guides

- ✅ Provided for major version upgrades
- ✅ Step-by-step instructions
- ✅ Examples and code snippets
- ✅ Rollback procedures

### Support Period

- **Active Support**: 3 months after release
- **Security Support**: 6 months after next major release
- **Community Support**: Indefinitely via GitHub

## Exception Policy

### Security Fixes

Security fixes may require breaking changes:
- ✅ Applied immediately to current version
- ✅ Backported to supported versions
- ✅ Clearly documented
- ✅ Migration guide provided if needed

### Critical Bug Fixes

Critical bug fixes may require behavioral changes:
- ✅ Clearly documented
- ✅ Migration guide if breaking
- ✅ Deprecation warnings if applicable

## Questions?

- **Documentation**: https://docs.mockforge.dev
- **Issues**: https://github.com/SaaSy-Solutions/mockforge/issues
- **Discussions**: https://github.com/SaaSy-Solutions/mockforge/discussions

---

**Last Updated**: 2025-01-27
**Version**: 1.0.0
