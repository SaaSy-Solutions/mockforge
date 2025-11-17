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

## [0.3.0] - 2025-11-17

### Added

- **[DevX] Pillars & Tagged Changelog**: Complete pillar system implementation with documentation and tooling
- **[Reality] Smart Personas & Reality Continuum v2**: Complete persona graph and lifecycle system
- **[Contracts] Drift Budget & GitOps for API Sync**: Complete drift management system
- **[Reality] Behavioral Cloning v1**: Multi-step flow recording and replay
- **[AI][DevX] LLM/Voice Interface for Workspace Creation**: Natural language to complete workspace

See [CHANGELOG.md](../../../CHANGELOG.md) for detailed release notes.

## [0.2.0] - 2025-10-29

### Added

- **[DevX] Output control features** for MockForge generator with comprehensive configuration options
- **[DevX] Unified spec parser** with enhanced validation and error reporting
- **[DevX] Multi-framework client generation** with Angular and Svelte support
- **[Reality] Enhanced mock data generation** with OpenAPI support
- **[DevX] Configuration file support** for mock generation
- **[DevX] Browser mobile proxy mode** implementation
- **[DevX] Comprehensive documentation** and example workflows

### Changed

- **[DevX] Enhanced CLI** with progress indicators, error handling, and code quality improvements
- **[DevX] Comprehensive plugin architecture documentation**

### Fixed

- **[DevX] Remove tests that access private fields** in mock data tests
- **[DevX] Fix compilation issues** in mockforge-collab and mockforge-ui
- **[DevX] Update mockforge-plugin-core version** to 0.1.6 in plugin-sdk
- **[DevX] Enable SQLx offline mode** for mockforge-collab publishing
- **[DevX] Add description field** to mockforge-analytics
- **[DevX] Add version requirements** to all mockforge path dependencies
- **[DevX] Fix publish order dependencies** (mockforge-chaos before mockforge-reporting)
- **[DevX] Update Cargo.lock** and format client generator tests

## [0.1.3] - 2025-10-22

### Changes

- **[DevX] docs: prepare release 0.1.3**
- **[DevX] docs: update CHANGELOG for 0.1.3 release**
- **[DevX] docs: add roadmap completion summary**
- **[DevX] feat: add Kubernetes-style health endpoint aliases and dashboard shortcut**
- **[DevX] feat: add unified config & profiles with multi-format support**
- **[Reality] feat: add capture scrubbing and deterministic replay**
- **[DevX] feat: add native GraphQL operation handlers with advanced features**
- **[Reality] feat: add programmable WebSocket handlers**
- **[Reality] feat: add HTTP scenario switching for OpenAPI response examples**
- **[DevX] feat: add mockforge-test crate and integration testing examples**
- **[DevX] build: enable publishing for mockforge-ui and mockforge-cli**
- **[DevX] build: extend publish script for internal crates**
- **[DevX] build: parameterize publish script with workspace version**

## [0.1.2] - 2025-10-17

### Changes

- **[DevX] build: make version update tolerant**
- **[DevX] build: manage version references via wrapper**
- **[DevX] build: mark example crates as non-publishable**
- **[DevX] build: drop publish-order for cargo-release 0.25**
- **[DevX] build: centralize release metadata in release.toml**
- **[DevX] build: remove per-crate release metadata**
- **[DevX] build: fix release metadata field name**
- **[DevX] build: move workspace release metadata into Cargo.toml**
- **[DevX] build: require execute flag for release wrapper**
- **[DevX] build: automate changelog generation during release**
- **[DevX] build: add release wrapper with changelog guard**
- **[DevX] build: align release tooling with cargo-release 0.25**

## [0.1.1] - 2025-10-17

### Added

- **[Contracts] OpenAPI request validation** (path/query/header/cookie/body) with deep $ref resolution and composite schemas (oneOf/anyOf/allOf).
- **[Contracts] Validation modes**: `disabled`, `warn`, `enforce`, with aggregate error reporting and detailed error objects.
- **[DevX] Runtime Admin UI panel** to view/toggle validation mode and per-route overrides; Admin API endpoint `/__mockforge/validation`.
- **[DevX] CLI flags and config options** to control validation (including `skip_admin_validation` and per-route `validation_overrides`).
- **[DevX] New e2e tests** for 2xx/422 request validation and response example expansion across HTTP routes.
- **[DevX] Templating reference docs** and examples; WS templating tests and demo update.
- **[Reality] Initial release of MockForge** - Multi-protocol mocking framework
- **[Reality] HTTP API mocking** with OpenAPI support
- **[Reality] gRPC service mocking** with Protocol Buffers
- **[Reality] WebSocket connection mocking** with replay functionality
- **[DevX] CLI tool** for easy local development
- **[DevX] Admin UI** for managing mock servers
- **[DevX] Comprehensive documentation** with mdBook
- **[DevX] GitHub Actions CI/CD pipeline**
- **[DevX] Security audit integration**
- **[DevX] Pre-commit hooks** for code quality

### Changed

- **[Contracts] HTTP handlers now perform request validation** before routing; invalid requests return 400 with structured details (when `enforce`).
- **[Contracts] Bump `jsonschema` to 0.33** and adapt validator API; enable draft selection and format checks internally.
- **[Contracts] Improve route registry and OpenAPI parameter parsing**, including styles/explode and array coercion for query/header/cookie parameters.

### Deprecated

- N/A

### Removed

- N/A

### Fixed

- **[DevX] Resolve admin mount prefix** from config and exclude admin routes from validation when configured.
- **[Contracts] Various small correctness fixes** in OpenAPI schema mapping and parameter handling; clearer error messages.

### Security

- N/A
