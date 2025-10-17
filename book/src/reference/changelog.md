## [Unreleased]

### Added

- Nothing yet.

### Changed

- Nothing yet.

### Deprecated

- Nothing yet.

### Removed

- Nothing yet.

### Fixed

- Nothing yet.

### Security

- Nothing yet.

## [0.1.2] - 2025-10-17

### Changes

- build: mark example crates as non-publishable
- build: drop publish-order for cargo-release 0.25
- build: centralize release metadata in release.toml
- build: remove per-crate release metadata
- build: fix release metadata field name
- build: move workspace release metadata into Cargo.toml
- build: require execute flag for release wrapper
- build: automate changelog generation during release
- build: add release wrapper with changelog guard
- build: align release tooling with cargo-release 0.25

## [0.1.2] - 2025-10-17

### Changes

- build: mark example crates as non-publishable
- build: drop publish-order for cargo-release 0.25
- build: centralize release metadata in release.toml
- build: remove per-crate release metadata
- build: fix release metadata field name
- build: move workspace release metadata into Cargo.toml
- build: require execute flag for release wrapper
- build: automate changelog generation during release
- build: add release wrapper with changelog guard
- build: align release tooling with cargo-release 0.25

## [0.1.2] - 2025-10-17

### Changes

- build: drop publish-order for cargo-release 0.25
- build: centralize release metadata in release.toml
- build: remove per-crate release metadata
- build: fix release metadata field name
- build: move workspace release metadata into Cargo.toml
- build: require execute flag for release wrapper
- build: automate changelog generation during release
- build: add release wrapper with changelog guard
- build: align release tooling with cargo-release 0.25

## [0.1.2] - 2025-10-17

### Changes

- build: centralize release metadata in release.toml
- build: remove per-crate release metadata
- build: fix release metadata field name
- build: move workspace release metadata into Cargo.toml
- build: require execute flag for release wrapper
- build: automate changelog generation during release
- build: add release wrapper with changelog guard
- build: align release tooling with cargo-release 0.25

## [0.1.2] - 2025-10-17

### Changes

- build: remove per-crate release metadata
- build: fix release metadata field name
- build: move workspace release metadata into Cargo.toml
- build: require execute flag for release wrapper
- build: automate changelog generation during release
- build: add release wrapper with changelog guard
- build: align release tooling with cargo-release 0.25

## [0.1.2] - 2025-10-17

### Changes

- build: fix release metadata field name
- build: move workspace release metadata into Cargo.toml
- build: require execute flag for release wrapper
- build: automate changelog generation during release
- build: add release wrapper with changelog guard
- build: align release tooling with cargo-release 0.25

## [0.1.2] - 2025-10-17

### Changes

- build: move workspace release metadata into Cargo.toml
- build: require execute flag for release wrapper
- build: automate changelog generation during release
- build: add release wrapper with changelog guard
- build: align release tooling with cargo-release 0.25

## [0.1.2] - 2025-10-17

### Changes

- build: require execute flag for release wrapper
- build: automate changelog generation during release
- build: add release wrapper with changelog guard
- build: align release tooling with cargo-release 0.25

## [0.1.2] - 2025-10-17

### Changes

- build: require execute flag for release wrapper
- build: automate changelog generation during release
- build: add release wrapper with changelog guard
- build: align release tooling with cargo-release 0.25

## [0.1.1] - 2025-10-17

### Added

- OpenAPI request validation (path/query/header/cookie/body) with deep $ref resolution and composite schemas (oneOf/anyOf/allOf).
- Validation modes: `disabled`, `warn`, `enforce`, with aggregate error reporting and detailed error objects.
- Runtime Admin UI panel to view/toggle validation mode and per-route overrides; Admin API endpoint `/__mockforge/validation`.
- CLI flags and config options to control validation (including `skip_admin_validation` and per-route `validation_overrides`).
- New e2e tests for 2xx/422 request validation and response example expansion across HTTP routes.
- Templating reference docs and examples; WS templating tests and demo update.
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

- HTTP handlers now perform request validation before routing; invalid requests return 400 with structured details (when `enforce`).
- Bump `jsonschema` to 0.33 and adapt validator API; enable draft selection and format checks internally.
- Improve route registry and OpenAPI parameter parsing, including styles/explode and array coercion for query/header/cookie parameters.

### Deprecated

- N/A

### Removed

- N/A

### Fixed

- Resolve admin mount prefix from config and exclude admin routes from validation when configured.
- Various small correctness fixes in OpenAPI schema mapping and parameter handling; clearer error messages.

### Security

- N/A
