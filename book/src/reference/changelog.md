## [Unreleased]

### Added

- **[Reality] Reality Profiles Marketplace**: Pre-tuned "realism packs" bundling personas, scenarios, chaos rules, latency curves, error distributions, data mutation, and protocol behaviors. Includes E-Commerce Peak Season, Fintech Fraud, Healthcare HL7, and IoT Device Fleet Chaos packs.
- **[Reality] Behavioral Economics Engine**: Mocks react to pressure, load, pricing, fraud suspicion, and customer segments. Supports declarative and scriptable rules for cart conversion drops, transaction declines, and user churn simulation.
- **[Reality] Synthetic → Recorded Drift Learning**: Mocks learn from traffic patterns and adapt behavior. Supports behavioral, statistical, and hybrid learning modes with configurable sensitivity and decay rates.
- **[Reality] World State Engine**: Unified visualization of all MockForge state systems (personas, lifecycle, reality, time, multi-protocol state, behavior trees, generative schemas, recorded data, AI modifiers) with graph visualization and time travel capabilities.
- **[Reality] Performance Mode**: Lightweight load simulation for running scenarios at N RPS, simulating bottlenecks, recording latencies, and observing response changes under load.
- **[Contracts] API Change Forecasting**: Predicts likely future contract breaks based on historical drift patterns. Includes pattern analysis, statistical modeling, and multi-window forecasting (30/90/180 days).
- **[Contracts] Semantic Drift Notifications**: Detects when the meaning of an API changes, not just structure. Includes description changes, enum narrowing, soft-breaking changes, nullable changes, and error code removals with LLM-powered analysis.
- **[Contracts] Contract Threat Modeling**: Analyzes APIs for security risks including PII exposure, DoS risk (unbounded arrays), error leakage (stack traces), and schema design issues with AI-powered remediation suggestions.
- **[DevX] Zero-Config Mode (Runtime Daemon)**: Automatically creates mocks, generates types, creates client stubs, updates OpenAPI schema, and sets up scenarios when hitting non-existent endpoints (404). Full integration with AI generation.
- **[DevX] DevTools Browser Integration**: Enhanced ForgeConnect extension with DevTools panel featuring "Mock this endpoint" functionality, live response modification, persona/scenario toggling, reverse-injection into workspace, and snapshot diff visualization.
- **[DevX] Snapshot Diff Between Environments**: Side-by-side visualization for comparing mock behavior between environments, personas, scenarios, or reality levels. Supports test vs prod, persona comparisons, and reality level comparisons.
- **[DevX] Mock-Oriented Development (MOD)**: Complete methodology documentation for mock-first design, contract-driven development, reality progression, and scenario-driven testing.
- **[Cloud] MockOps Pipelines**: GitHub Actions-like automation for mock lifecycle management. Event-driven pipelines for schema changes → auto-regenerate SDK, scenario promotion, and drift threshold → auto-generate Git PR.
- **[Cloud] Multi-Workspace Federation**: Compose multiple mock workspaces into one federated "virtual system" for microservices architectures. Supports service boundaries, system-wide scenarios, and per-service reality level control.
- **[Cloud] Analytics Dashboard**: Leadership insight into coverage, risk, and usage. Includes scenario usage heatmaps, persona CI hit tracking, endpoint coverage analysis, reality level staleness detection, and drift percentage tracking.
- **[AI] API Architecture Critique**: LLM-powered analysis of API schemas to detect anti-patterns, redundancies, naming issues, emotional tone problems, and provide restructuring recommendations.
- **[AI] Natural Language to System Generation**: Generate complete backend systems from natural language descriptions. Creates 20-30 REST endpoints, 4-5 personas, 6-10 lifecycle states, WebSocket topics, payment failure scenarios, surge pricing chaos profiles, full OpenAPI spec, GraphQL schema, typings, and CI pipeline templates.
- **[AI] AI Behavioral Simulation Engine**: Models users as narrative agents that react to app state, form intentions (shop, browse, buy, abandon), respond to errors, and trigger multi-step interactions automatically.

### Changed

- **[DevX] Enhanced ForgeConnect SDK Documentation**: Added comprehensive DevTools panel features including "Mock this endpoint" functionality, live response modification, persona/scenario toggling, reverse-injection, and snapshot diff visualization.

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
