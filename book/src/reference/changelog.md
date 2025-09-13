# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
