# MockForge Architecture

This document outlines the crate structure, dependency graph, and public vs. internal API boundaries for MockForge.

## Crate Structure

MockForge is organized as a Cargo workspace with multiple crates. The crates fall into three categories:

### Public API Crates (Published to crates.io)

These crates are intended for use by external developers and plugin authors:

#### Core Libraries
- **`mockforge-core`**: Core types, routing, validation, latency simulation, and proxy logic. Foundation for all protocol implementations.
- **`mockforge-http`**: HTTP/REST protocol support with OpenAPI integration
- **`mockforge-ws`**: WebSocket protocol support
- **`mockforge-grpc`**: gRPC protocol support with protobuf reflection
- **`mockforge-graphql`**: GraphQL protocol support
- **`mockforge-data`**: Synthetic data generation with faker and RAG capabilities

#### Plugin Development
- **`mockforge-plugin-core`**: Core plugin interfaces, types, and WASM runtime for the plugin system
  - **Target audience**: Plugin developers
  - **Stability requirement**: High - breaking changes impact all plugins
  - **Documentation requirement**: Comprehensive (enforced with `missing_docs = "deny"`)

- **`mockforge-plugin-sdk`**: Helper macros, testing utilities, and convenience wrappers for plugin development
  - **Target audience**: Plugin developers
  - **Stability requirement**: High - convenience layer for plugin authors
  - **Documentation requirement**: Comprehensive (enforced with `missing_docs = "deny"`)

- **`mockforge-plugin-loader`**: Plugin loading, security sandboxing, and validation
  - **Target audience**: Advanced users integrating MockForge into custom tools
  - **Use case**: Custom plugin management solutions

### Internal Crates (Not Published - `publish = false`)

These crates are implementation details and not meant for external consumption:

#### Binaries
- **`mockforge-cli`**: Main CLI binary for running MockForge servers
- **`mockforge-plugin-cli`**: CLI tool for plugin development (scaffolding, building, testing)

#### Internal Components
- **`mockforge-ui`**: Web-based admin UI for managing mock servers
  - Not meant for reuse as a library
  - Tightly coupled to CLI implementation

- **`mockforge-recorder`**: Request/response recording and replay functionality
  - Integrated into CLI, not standalone

- **`mockforge-observability`**: Prometheus metrics and observability features
  - Internal metrics collection

- **`mockforge-tracing`**: OpenTelemetry and distributed tracing integration
  - Internal tracing infrastructure

- **`mockforge-chaos`**: Chaos engineering features (fault injection, circuit breakers)
  - Internal testing and resilience features

- **`mockforge-reporting`**: Report generation and visualization
  - Internal reporting tools

- **`mockforge-plugin-registry`**: Plugin registry client for discovering plugins
  - Internal plugin discovery mechanism

### Test/Example Crates
- **`test_openapi_demo`**: Integration test demonstrating OpenAPI features
- **`examples/plugins/*`**: Example plugin implementations

## Dependency Graph

The dependency structure follows a clean layered architecture with no circular dependencies:

```
┌─────────────────────────────────────────────────────────────┐
│                     Binary Layer                            │
│  mockforge-cli, mockforge-plugin-cli                       │
│  (depend on everything needed for their functionality)      │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                  Protocol & Feature Layer                   │
│  mockforge-http, mockforge-ws, mockforge-grpc,             │
│  mockforge-graphql, mockforge-ui, mockforge-recorder       │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                  Plugin & Extension Layer                   │
│  mockforge-plugin-loader, mockforge-plugin-sdk             │
│                    ↓                                        │
│            mockforge-plugin-core                            │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                     Foundation Layer                        │
│  mockforge-core (shared types, routing, validation)        │
│  mockforge-data (data generation)                           │
│  mockforge-observability, mockforge-tracing                 │
└─────────────────────────────────────────────────────────────┘
```

### Key Dependency Rules

1. **No circular dependencies**: All dependencies flow downward in the layer hierarchy
2. **Core is foundational**: `mockforge-core` is dependency-free (except for common libraries) and provides shared types
3. **Binaries pull in features**: Only CLI binaries (`mockforge-cli`, `mockforge-plugin-cli`) depend on multiple protocol implementations
4. **Plugin isolation**: Plugin crates (`mockforge-plugin-core`, `-sdk`, `-loader`) form an independent subsystem
5. **Protocol independence**: HTTP, WebSocket, gRPC, and GraphQL implementations are independent of each other

## For Plugin Developers

If you're writing a MockForge plugin:

1. **Required dependency**: `mockforge-plugin-core` - Core plugin traits and types
2. **Recommended dependency**: `mockforge-plugin-sdk` - Helper macros and testing utilities
3. **Do not depend on**: Internal crates (those marked with `publish = false`)

### Plugin Development Resources

- Plugin API documentation: https://docs.rs/mockforge-plugin-core
- Plugin SDK documentation: https://docs.rs/mockforge-plugin-sdk
- Plugin development guide: See `docs/plugin-development.md`
- Example plugins: See `examples/plugins/` directory

## For MockForge Integrators

If you're integrating MockForge into your own tools or services:

1. **Core mock functionality**: Depend on `mockforge-core` and the specific protocol crates you need
   - Example: `mockforge-http` for HTTP mocking, `mockforge-grpc` for gRPC
2. **Data generation**: Depend on `mockforge-data` for synthetic data
3. **Plugin support**: Depend on `mockforge-plugin-loader` for plugin management
4. **Do not depend on**: CLI binaries or internal implementation crates

## Stability Guarantees

### Before 1.0 Release

All crates follow semantic versioning with the `0.x.y` convention:
- Minor version bumps (0.1 → 0.2) may include breaking changes
- Patch version bumps (0.1.0 → 0.1.1) are backwards compatible

### After 1.0 Release

Public API crates will follow strict semantic versioning:
- **Major version** (1.0 → 2.0): Breaking changes allowed
- **Minor version** (1.0 → 1.1): Backwards-compatible additions only
- **Patch version** (1.0.0 → 1.0.1): Backwards-compatible bug fixes only

Special stability considerations:
- **`mockforge-plugin-core`**: Highest stability requirement - breaking changes are avoided when possible
- **`mockforge-plugin-sdk`**: May evolve faster with deprecation warnings for breaking changes

## Contributing

When adding new crates or modifying dependencies:

1. Ensure no circular dependencies are introduced
2. Mark internal-only crates with `publish = false` in `Cargo.toml`
3. Public API crates must have comprehensive documentation (enforced by `missing_docs` lint)
4. Update this document when adding new crates or changing the architecture
5. Run `cargo tree` to verify the dependency graph remains clean

## Questions?

For questions about architecture decisions or crate organization:
- File an issue: https://github.com/SaaSy-Solutions/mockforge/issues
- Discussions: https://github.com/SaaSy-Solutions/mockforge/discussions
