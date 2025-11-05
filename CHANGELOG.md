## [0.2.5] - 2025-01-27

### Added

- **OAuth2 Flow Support**: Complete OAuth2 implementation with all standard flows
  - Authorization Code flow with PKCE (RFC 7636 compliant, SHA256 hash)
  - Client Credentials flow for server-side applications
  - Password flow for trusted clients
  - Implicit flow support
  - Automatic token refresh and expiration management
  - State parameter for CSRF protection
  - PKCE code verifier/challenge generation helpers
  - Token storage with expiration tracking (localStorage)

- **Enterprise Error Handling**: Structured error handling for generated clients
  - `ApiError` class with status codes, statusText, and error body
  - `RequiredError` class for missing required fields
  - Helper methods: `isClientError()`, `isServerError()`, `getErrorDetails()`, `getVerboseMessage()`
  - Optional verbose error messages with detailed validation information

- **Request/Response Validation**: Built-in validation support
  - Required field validation before sending requests
  - Basic response structure validation (type checking, object validation)
  - Configurable via `validateRequests` flag
  - Detailed validation error messages

- **Request/Response Interceptors**: Custom request/response/error transformation
  - Request interceptor: Modify requests before sending
  - Response interceptor: Transform responses after receiving
  - Error interceptor: Global error handling
  - Support for async interceptors

- **Enhanced Authentication**: Multiple authentication methods
  - Bearer token (static or dynamic function)
  - API key authentication (static or dynamic)
  - Basic authentication (username/password)
  - OAuth2 (all flows, takes priority over other methods)

- **PKCE Helper Functions**: Exported utilities for PKCE implementation
  - `generatePKCECodeVerifier()`: Generate cryptographically random code verifier
  - `generatePKCECodeChallenge()`: Generate SHA256 code challenge from verifier

- **Security Best Practices**: Comprehensive security warnings and guidance
  - Client secret warnings for browser-based applications
  - XSS vulnerability warnings for localStorage token storage
  - CSRF protection via state parameter validation
  - Token expiration checking
  - Security documentation in generated README

- **Request Timeout Handling**: Configurable request timeouts
  - Default 30-second timeout (configurable)
  - AbortController-based timeout implementation
  - Proper timeout error handling

- **React Query Integration Documentation**: Comprehensive examples for @tanstack/react-query integration

### Changed

- **React Client Generator**: Major enhancements to generated React client code
  - Replaced placeholder PKCE implementation with full SHA256-based solution
  - Implemented proper response validation (previously placeholder)
  - Enhanced README with comprehensive feature documentation
  - Improved error messages and validation details
  - Better security documentation and best practices

- **Operation ID Sanitization**: Improved identifier generation
  - Enhanced `sanitize_identifier` function to handle complex operation IDs
  - Better handling of parentheses, slashes, hyphens in operation IDs
  - Proper camelCase conversion with word boundary detection

### Fixed

- **TypeScript Empty Object Types**: Fixed formatting issue where empty object schemas generated invalid TypeScript
  - Empty objects now correctly generate as `[key: string]: any;` instead of malformed `Record<string, any>}`

- **DELETE Operations with Query Params**: Fixed missing query parameter support in DELETE operations

- **Duplicate Operation IDs**: Fixed duplicate operation ID handling by appending numeric suffixes

- **PKCE Code Challenge**: Fixed PKCE implementation to use proper SHA256 hash instead of plain encoding

- **Response Validation**: Replaced placeholder with actual implementation (type checking, structure validation)

### Security

- Added comprehensive security warnings for OAuth2 client secrets in browser code
- Added XSS vulnerability warnings for localStorage token storage
- Implemented CSRF protection via state parameter validation
- Added token expiration checking to prevent use of expired tokens
- Documented security best practices in generated client README

## [0.2.6] - 2025-11-04

### Added

- **TLS/HTTPS and mTLS Support**: Added TLS/HTTPS and mutual TLS (mTLS) support for HTTP server
  - Configurable TLS certificate and key paths
  - Client certificate authentication support
  - Secure connection handling for production deployments

- **Built-in Tunneling Service**: Added built-in tunneling service for exposing local servers via public URLs
  - Automatic tunnel creation for local development
  - Public URL generation for testing and demos
  - Integration with popular tunneling services

- **SDK Implementation**: Completed Phase 1 & 2 of SDK implementation
  - Comprehensive documentation and examples
  - Production-ready client generators

### Changed

- **Version Bumps**: Updated all workspace crates from 0.2.5 to 0.2.6
  - Updated all dependency versions across the workspace
  - Fixed version mismatches in mockforge-ui and mockforge-plugin-loader

- **Publishing Improvements**: Enhanced crate publishing process
  - Added mockforge-tcp and mockforge-test to publish script
  - Enabled publishing for mockforge-test crate
  - Fixed mockforge-tcp to remove README requirement

### Fixed

- **Documentation**: Fixed missing module-level documentation in test files
  - Added comprehensive module documentation to all test modules
  - Improved code documentation consistency

- **Axum Compatibility**: Fixed Axum 0.8 compatibility issues in proxy server module
  - Updated proxy server to work with latest Axum version
  - Resolved breaking changes from Axum upgrade

- **MQTT Error Types**: Fixed MQTT publish handlers error types to be Send + Sync
  - Updated error types for proper async/await compatibility
  - Ensured thread-safety in MQTT handlers

## [0.2.7] - 2025-11-05

### Added

- **Automatic API Sync & Change Detection**: Implemented periodic polling and automatic sync for detecting upstream API changes
  - Periodic sync service with configurable intervals (default: 1 hour)
  - Automatic change detection using deep response comparison (status, headers, body)
  - Optional automatic fixture updates when changes detected
  - Manual sync trigger via API (`POST /api/recorder/sync/now`)
  - Sync status tracking and change history
  - Configurable sync settings: upstream URL, interval, headers, timeout, max requests
  - Support for GET-only or all-methods sync
  - Detailed change reports with before/after comparisons
  - Database update method for refreshing recorded responses
  - API endpoints: `/api/recorder/sync/status`, `/api/recorder/sync/config`, `/api/recorder/sync/changes`

- **TCP Protocol Support**: Added raw TCP server mocking support via new `mockforge-tcp` crate
  - Raw TCP connection handling with fixture-based matching
  - Echo mode for testing TCP clients
  - TLS/SSL support for encrypted connections
  - Delimiter-based message framing (optional)
  - Configurable buffer sizes and connection limits
  - CLI flag `--tcp-port` for custom TCP server port
  - Configuration via `config.tcp` in YAML/JSON config files

- **Response Selection Modes**: Added support for sequential (round-robin) and random response selection when multiple examples are available
  - Sequential mode: Cycles through available examples in order (round-robin)
  - Random mode: Randomly selects from available examples
  - Weighted random mode: Random selection with custom weights per example
  - Configuration via `x-mockforge-response-selection` OpenAPI extension
  - Environment variable support: `MOCKFORGE_RESPONSE_SELECTION_MODE` (global) and `MOCKFORGE_RESPONSE_SELECTION_<OPERATION_ID>` (per-operation)
  - State tracking for sequential mode ensures round-robin behavior across requests

- **Webhook HTTP Execution**: Implemented actual HTTP request execution in chaos orchestration hooks
  - `HookAction::HttpRequest` now executes real outbound HTTP requests (previously only logged)
  - Supports GET, POST, PUT, DELETE, PATCH methods
  - Configurable request body and headers
  - Error handling and logging for webhook failures
  - Fire-and-forget execution (failures don't block orchestration)

- **CRUD & Webhook Documentation**: Added comprehensive documentation guides
  - `docs/CRUD_SIMULATION.md`: Complete guide for simulating CRUD operations with stateful data store
  - `docs/WEBHOOKS_CALLBACKS.md`: Full documentation of webhook capabilities via hooks, chains, and scripts
  - Examples demonstrating realistic workflows and integrations

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

## [0.2.4] - 2025-01-27

### Fixed

- Fix request body parameter generation in React/Vue/Svelte client generators - request bodies now correctly generate `data` parameter and `body: JSON.stringify(data)` in API client methods
- Fix required vs optional field handling in generated TypeScript interfaces - required fields no longer incorrectly marked with optional marker (`?`)
- Fix OpenAPI serde deserialization by adding `#[serde(rename)]` attributes for `operationId` and `requestBody` fields
- Apply required fields processing consistently across all client generators (React, Vue, Svelte)

### Added

- Comprehensive test coverage for request body parameter scenarios (POST, PUT, PATCH, DELETE)
- Test cases for `$ref` schemas in request bodies
- Test cases for YAML spec support verification

## [0.2.3] - 2025-01-27

### Fixed

- Fix OpenAPI example extraction to prioritize explicit examples from schema and properties
- Fix request body parameter generation in React client generator for POST, PUT, PATCH, DELETE methods
- Fix Handlebars template logic for request body type generation in client code
- Fix useCallback dependency array formatting in React hooks template
- Add comprehensive test coverage for request body parameter scenarios

## [0.2.0] - 2025-10-29

### Added

- Output control features for MockForge generator with comprehensive configuration options
- Unified spec parser with enhanced validation and error reporting
- Multi-framework client generation with Angular and Svelte support
- Enhanced mock data generation with OpenAPI support
- Configuration file support for mock generation
- Browser mobile proxy mode implementation
- Comprehensive documentation and example workflows

### Changed

- Enhanced CLI with progress indicators, error handling, and code quality improvements
- Comprehensive plugin architecture documentation

### Fixed

- Remove tests that access private fields in mock data tests
- Fix compilation issues in mockforge-collab and mockforge-ui
- Update mockforge-plugin-core version to 0.1.6 in plugin-sdk
- Enable SQLx offline mode for mockforge-collab publishing
- Add description field to mockforge-analytics
- Add version requirements to all mockforge path dependencies
- Fix publish order dependencies (mockforge-chaos before mockforge-reporting)
- Update Cargo.lock and format client generator tests

## [0.1.3] - 2025-10-22

### Changes

- docs: prepare release 0.1.3
- docs: update CHANGELOG for 0.1.3 release
- docs: add roadmap completion summary
- feat: add Kubernetes-style health endpoint aliases and dashboard shortcut
- feat: add unified config & profiles with multi-format support
- feat: add capture scrubbing and deterministic replay
- feat: add native GraphQL operation handlers with advanced features
- feat: add programmable WebSocket handlers
- feat: add HTTP scenario switching for OpenAPI response examples
- feat: add mockforge-test crate and integration testing examples
- build: enable publishing for mockforge-ui and mockforge-cli
- build: extend publish script for internal crates
- build: parameterize publish script with workspace version

## [0.1.3] - 2025-10-22

### Changes

- docs: update CHANGELOG for 0.1.3 release
- docs: add roadmap completion summary
- feat: add Kubernetes-style health endpoint aliases and dashboard shortcut
- feat: add unified config & profiles with multi-format support
- feat: add capture scrubbing and deterministic replay
- feat: add native GraphQL operation handlers with advanced features
- feat: add programmable WebSocket handlers
- feat: add HTTP scenario switching for OpenAPI response examples
- feat: add mockforge-test crate and integration testing examples
- build: enable publishing for mockforge-ui and mockforge-cli
- build: extend publish script for internal crates
- build: parameterize publish script with workspace version

## [0.1.2] - 2025-10-17

### Changes

- build: make version update tolerant
- build: manage version references via wrapper
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
