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

## [0.3.21] - 2025-12-31

### Fixed

- **[DevX]** fix(bench): use custom flow config and fix sequential mode path matching - enables cross-resource dependency chains where one API resource depends on values from another
- **[DevX]** fix(bench): process dynamic placeholders in CRUD flow params file bodies (#79)
- chore: update benchmark baseline [skip ci]
- chore: enable publishing for previously internal crates
- chore: update benchmark baseline [skip ci]
- fix(release): disable sccache for crates.io publish
- chore: update benchmark baseline [skip ci]
- fix(release): publish all crates in dependency order
- fix(release): add mockforge-core to crates.io publish order
- chore: update benchmark baseline [skip ci]
- feat(bench): add --base-path option for API base path support (#79)
- chore: update benchmark baseline [skip ci]
- fix(collab): include SQLx query cache for crates.io installation (#79)
- chore: update benchmark baseline [skip ci]
- feat: implement optional enhancements from improvement plan
- fix: update doc tests to use rust,ignore for external dependencies
- chore: update benchmark baseline [skip ci]
- chore: add missing crates to workspace and restore path dependencies
- chore: restore path dependencies after publishing remaining v0.3.17 crates
- fix: restore all crates to workspace members list
- chore: restore path dependencies after publishing v0.3.17
- docs: update CHANGELOG for v0.3.17 release
- feat(bench): add WAFBench YAML integration for security testing
- Bump version to 0.3.17
- feat: comprehensive improvements across AMQP, MQTT, gRPC, registry server, and UI
- feat(ui): add type safety, mobile layout fixes, and search/filter to frontend
- Restore path dependencies after publishing v0.3.16
- Bump version to 0.3.16
- fix: resolve flaky tests and race conditions across test suite
- fix: replace panic-prone unwrap calls with safe error handling
- fix: resolve UUID storage format mismatch in collab crate tests
- Add multi-spec support and cross-spec dependency detection for bench command
- feat: add multi-spec support and cross-spec dependency handling to bench command
- fix: add validation to CRUD flow script generation
- fix: sanitize k6 CRUD flow metric names (#79 follow-up)
- Bump version to 0.3.13 and improve changelog
- Bump version to 0.3.12 and publish to crates.io
- Bump version to 0.3.11 and publish to crates.io
- chore: update benchmark baseline [skip ci]
- feat: add --params-file option for custom parameter values in bench
- Bump version to 0.3.10 and publish to crates.io
- chore: update benchmark baseline [skip ci]
- fix: move insecureSkipTLSVerify to global k6 options (fixes --insecure)
- chore: update benchmark baseline [skip ci]
- fix: resolve k6 bench issues with --insecure flag, textSummary, and query params
- chore: update benchmark baseline [skip ci]
- chore: bump version to 0.3.9 and update changelog
- feat: implement comprehensive mock server functionality across all crates
- chore: commit remaining version updates
- fix: enable publishing for mockforge-ui
- fix: enable publishing for mockforge-tunnel
- fix: update all 0.3.7 dependencies to 0.3.8 with path dependencies
- fix: add path dependencies for all workspace crates
- chore: update CHANGELOG date for 0.3.8
- chore: bump version to 0.3.8
- Fix cargo publish issues: add version requirements to dependencies
- chore: update benchmark baseline [skip ci]
- Apply formatting and additional code changes
- Fix compilation errors: update dependencies and adapt to API changes
- fix: remove path from mockforge-pipelines dep in mockforge-collab
- Add mockforge-sdk, mockforge-ui, mockforge-cli to workspace
- fix: add mockforge to restore function targets list
- fix: convert mockforge dev-dependencies to path dependencies
- fix: add mockforge-core to restore list and manually fix dependency
- fix: include mockforge-core in restore list
- fix: restore function now properly handles table-form dependencies without path
- fix: automatically restore dependencies at start of publish
- fix: restore all crate dependencies, not just a few
- fix: only convert dependencies for already-published crates
- fix: correct publish order - publish mockforge-data before mockforge-core
- fix: add mockforge-data as optional dependency in mockforge-core
- chore: bump version to 0.3.6 and update changelog
- chore: update benchmark baseline [skip ci]
- Fix k6 script generation and UI icon embedding issues
- chore: update benchmark baseline [skip ci]
- Add comprehensive test suite and fix build issues
- chore: update benchmark baseline [skip ci]
- docs: add comprehensive performance benchmarks documentation
- chore: update benchmark baseline [skip ci]
- fix: implement real functionality in benchmark tests and fix k8s-operator
- chore: update benchmark baseline [skip ci]
- fix: filter out 'change' directories from benchmark baseline parsing
- chore: update benchmark baseline [skip ci]
- chore: update benchmark baseline [skip ci]
- fix: GitHub Actions workflow cleanup and fixes (#81)
- chore: restore dependencies after publishing all crates
- fix: add mockforge-cli to workspace and add metadata to mockforge-k8s-operator
- fix: add missing crates to workspace (mockforge-sdk, mockforge-http, mockforge-ui, mockforge-k8s-operator)
- fix: add mockforge-world-state to workspace and publishing order before mockforge-http
- fix: add mockforge-route-chaos publishing step before mockforge-http
- fix: add mockforge-route-chaos to dependency targets and publishing order
- fix: add mockforge-route-chaos to workspace and publishing script
- fix: add mockforge-route-chaos to publishing order before mockforge-http
- fix: reduce keywords from 6 to 5 for mockforge-performance
- fix: reduce keywords to 5 for mockforge-performance (crates.io limit)
- fix: add mockforge-performance to publishing order before mockforge-http
- fix: add mockforge-collab to workspace members list
- fix: add mockforge-collab to workspace members
- fix: add missing README.md for mockforge-pipelines
- fix: add mockforge-pipelines to publishing order and dependency targets
- fix: add mockforge-pipelines to workspace and publishing script
- fix: add all missing crates to workspace members
- fix: handle short form dependencies when converting to path
- fix: publish mockforge-template-expansion before mockforge-core
- fix: add mockforge-template-expansion to publishing script
- fix: temporarily convert dependent crates' dependencies to path before publishing
- fix: remove argon2 from mockforge-core during MSRV checks
- fix: exclude mockforge-collab from MSRV checks and remove patch section
- fix: use awk instead of sed for multi-line patch section insertion
- fix: use Cargo patch section to pin base64ct for MSRV
- fix: improve base64ct pinning order in MSRV workflow
- fix: use exact version constraint for base64ct in MSRV workflow
- fix: improve base64ct pinning in MSRV workflow
- fix: pin base64ct to 1.7 for MSRV compatibility
- fix: exclude mockforge-ui from MSRV checks
- fix: add abd and existant to typos config
- fix: exclude FontAwesome and all minified files from spell check
- fix: also remove sysinfo from mockforge-ui during MSRV checks
- fix: exclude elasticlunr.min.js from spell check
- fix: exclude highlight.js from spell check
- fix: disable sysinfo feature during MSRV checks
- fix: sync sysinfo to 0.37, fix resolvable typo, exclude ace.js from spell check
- fix: pin sysinfo to 0.36, fix typos, improve MSRV workaround
- fix: update MSRV to 1.80 and add GraphQL exclusion workaround
- fix: update MSRV from 1.82 to 1.75
- fix: fix GitHub Actions workflow failures
- fix: standardize dependencies and fix all test failures
- Skip CRDs in kubectl validation to avoid server connection
- Fix kubectl validation to prevent server connection attempts
- Fix kubectl validation to skip server connection
- Fix all test failures and resolve dependency conflicts
- Fix k6 metric name validation error (issue #79) (#80)
- Optimize workflows: update deprecated actions and add path filters
- Fix mockforge-smtp version constraint from 0.2.0 to 0.3.3
- Fix Docker build, k8s validation, and spell check issues
- fix: update all mockforge dependency versions to 0.3.3 in mockforge-http
- chore: fix formatting (pre-commit hooks)
- deps(deps): bump opentelemetry_sdk from 0.21.2 to 0.31.0 (#67)
- chore: update benchmark baseline [skip ci]
- deps(deps): bump opentelemetry-semantic-conventions (#66)
- chore: update benchmark baseline [skip ci]
- deps(deps): bump sysinfo from 0.32.1 to 0.37.2 (#60)
- deps(deps): bump wasmparser from 0.239.0 to 0.240.0 (#64)
- deps(deps): bump governor from 0.6.3 to 0.8.1 (#61)
- chore: update benchmark baseline [skip ci]
- deps(deps): bump mail-parser from 0.9.4 to 0.11.1 (#63)
- deps(deps): bump rumqttc from 0.24.0 to 0.25.0 (#65)
- deps(deps): bump ndarray from 0.16.1 to 0.17.1 (#76)
- chore: update benchmark baseline [skip ci]
- ci(deps): bump azure/setup-helm from 3 to 4 (#72)
- ci(deps): bump actions/upload-artifact from 4 to 5 (#71)
- deps(deps): bump image from 0.24.9 to 0.25.9 (#77)
- deps(deps): bump rustls from 0.21.12 to 0.23.35 (#78)
- chore: update benchmark baseline [skip ci]
- Bump all crates to version 0.3.3
- Format code with rustfmt
- Fix k6 script generation with operation IDs containing dots/hyphens
- chore: update benchmark baseline [skip ci]
- perf: optimize template rendering by avoiding unnecessary operations
- chore: update benchmark baseline [skip ci]
- docs: update benchmark documentation with final optimizations
- perf: fix benchmark regressions and optimize measurements
- chore: update benchmark baseline [skip ci]
- Fix Kafka compilation errors and borrow checker issues
- feat: Implement cross-pillar enhancements - World State Engine, MOD, and Performance Mode
- feat(ai-studio): Add API Critique, System Generator, and Behavioral Simulator
- chore: rework UI/UX to be more AI native
- fix: Address pre-commit security vulnerabilities
- feat: Implement Invisible Mock Server experience (DevX Pillar)
- feat(security): implement email, Slack, and webhook notification services
- Refactor template expansion for Send safety
- chore: Restore path dependencies after 0.3.2 publish
- Fix: Complete SQLx query cache for mockforge-collab 0.3.2
- chore: update mockforge dependencies to version 0.3.1 across multiple crates
- fix: improve dependency conversion for optional dependencies and fix publishing order
- fix: update publish script to handle Phase 1 crate dependencies correctly
- feat: add comprehensive integration tests for 0.3.0 features and update changelog
- feat: Complete pillar enhancement gaps - VS Code extension and docs
- feat: Implement pillar tagging system and documentation enhancements
- feat: Implement MockForge AI Studio - Unified AI Copilot
- feat(cloud): Complete Cloud pillar implementation and fix compilation issues
- [DevX] Add JSON Schema support for config validation and IDE autocompletion
- feat: Implement Contract Fitness Functions, Consumer Impact Analysis, and Multi-Protocol Contracts
- feat: Enhance Reality feature with observability, cross-protocol consistency, and time-aware lifecycles
- fix: use proper vosk API by matching on CompleteResult enum
- fix: resolve all compilation errors
- chore: prepare release 0.3.0
- feat: Implement LLM Studio - Natural Language Workspace Creation (0.3.4)
- feat: Complete Behavioral Cloning v1 implementation and refactor architecture
- feat: Implement Drift Budget & GitOps for API Sync + AI Contract Diff
- feat: implement Scenario Studio Visual Editor with React Flow
- feat: implement AI-Native Interface Deepening features
- feat: Implement Time Travel & Snapshots and Frontend X-Ray Mode
- feat(sdk): Add Contract-Backed Types and Scenario-First SDKs to Vue, Svelte, and Angular
- Format code: Apply rustfmt and whitespace cleanup
- Release v0.2.9: Update version, CHANGELOG, and publish all crates to crates.io
- Add registry server improvements, password reset, metrics, and marketplace enhancements
- security: Upgrade wasmtime to 36.0.3 to fix RUSTSEC-2025-0118
- feat: Fix compilation errors and implement comprehensive E2E test suite
- fix: implement custom routes, template expansion, latency injection, and init improvements
- feat: Smart Personas with array generation and relationship inference
- feat: Complete Java and .NET SDK implementations with builder patterns
- fix: update all test files for new function signatures
- fix: resolve all compilation errors across workspace
- Complete Phase 3 security controls implementation
- Add cloud monetization infrastructure and features
- Implement organization management endpoints
- Fix Axum 0.8 route syntax in state_machine_api.rs
- Fix file server route syntax for Axum 0.8 compatibility
- Release v0.2.8: Publish all crates to crates.io
- chore: bump version to 0.2.8
- feat: Complete Generative Schema Mode and achieve 100% roadmap completion
- Implement Smart Personas feature for consistent cross-endpoint data generation
- Add Reality Continuum feature for blending mock and real data sources
- Implement Voice + LLM Interface with STT backends
- Implement complete Deceptive Deploy feature
- Add GraphQL + REST Playground with workspace filtering
- Implement ForgeConnect SDK with full feature set
- Add enhanced scenario marketplace features
- Configure SQLx and integrate mockforge-collab with mockforge-core
- Fix test compilation errors in reality integration and hot-reload tests
- Implement Reality Slider feature with hot-reload support
- Complete latency recording integration and fix WorkspaceConfig reality_level field
- style: Apply rustfmt formatting to Chaos Lab code
- feat: Add Chaos Lab interactive network condition simulation
- Fix test compilation errors in openapi_generator_tests
- Fix all compilation errors for AI Contract Diff feature
- Add WireMock-inspired features: browser proxy mode, git sync, data sources, template library, managed hosting docs, and user management
- Add comprehensive ecosystem and use cases documentation
- Complete configuration and extensibility implementation
- Add advanced behavior and simulation features
- Fix test and benchmark compilation errors
- Complete Scenario State Machines 2.0 with sub-scenario execution
- Implement VBR Engine enhancements: OpenAPI integration, M2M relationships, seeding, ID generation, snapshots
- Add mock-to-real migration pipeline with per-route toggling
- Add Data Scenarios Marketplace feature
- feat: Implement ForgeConnect - Front-End Integrated Mode for browser-based mock creation
- Add MockForge Cloud Graph visualization with real-time updates and export
- Add data personality profiles system for consistent mock data generation
- Add realistic network conditions and chaos lab with interactive UI controls
- Add temporal simulation with CLI commands and scenario support
- Complete MockAI implementation with query params and session recording
- Add Virtual Backend Reality (VBR) engine
- Add multipart form data support and file generation/serving for API mocks
- fix: update mockforge-plugin-sdk to use workspace version
- fix: enable publishing for mockforge-tunnel and add to publish script

## [0.3.21] - 2025-12-31

### Changes

- fix(bench): use custom flow config and fix sequential mode path matching
- chore: update benchmark baseline [skip ci]
- fix(bench): process dynamic placeholders in CRUD flow params file bodies (#79)
- chore: update benchmark baseline [skip ci]
- chore: enable publishing for previously internal crates
- chore: update benchmark baseline [skip ci]
- fix(release): disable sccache for crates.io publish
- chore: update benchmark baseline [skip ci]
- fix(release): publish all crates in dependency order
- fix(release): add mockforge-core to crates.io publish order
- chore: update benchmark baseline [skip ci]
- feat(bench): add --base-path option for API base path support (#79)
- chore: update benchmark baseline [skip ci]
- fix(collab): include SQLx query cache for crates.io installation (#79)
- chore: update benchmark baseline [skip ci]
- feat: implement optional enhancements from improvement plan
- fix: update doc tests to use rust,ignore for external dependencies
- chore: update benchmark baseline [skip ci]
- chore: add missing crates to workspace and restore path dependencies
- chore: restore path dependencies after publishing remaining v0.3.17 crates
- fix: restore all crates to workspace members list
- chore: restore path dependencies after publishing v0.3.17
- docs: update CHANGELOG for v0.3.17 release
- feat(bench): add WAFBench YAML integration for security testing
- Bump version to 0.3.17
- feat: comprehensive improvements across AMQP, MQTT, gRPC, registry server, and UI
- feat(ui): add type safety, mobile layout fixes, and search/filter to frontend
- Restore path dependencies after publishing v0.3.16
- Bump version to 0.3.16
- fix: resolve flaky tests and race conditions across test suite
- fix: replace panic-prone unwrap calls with safe error handling
- fix: resolve UUID storage format mismatch in collab crate tests
- Add multi-spec support and cross-spec dependency detection for bench command
- feat: add multi-spec support and cross-spec dependency handling to bench command
- fix: add validation to CRUD flow script generation
- fix: sanitize k6 CRUD flow metric names (#79 follow-up)
- Bump version to 0.3.13 and improve changelog
- Bump version to 0.3.12 and publish to crates.io
- Bump version to 0.3.11 and publish to crates.io
- chore: update benchmark baseline [skip ci]
- feat: add --params-file option for custom parameter values in bench
- Bump version to 0.3.10 and publish to crates.io
- chore: update benchmark baseline [skip ci]
- fix: move insecureSkipTLSVerify to global k6 options (fixes --insecure)
- chore: update benchmark baseline [skip ci]
- fix: resolve k6 bench issues with --insecure flag, textSummary, and query params
- chore: update benchmark baseline [skip ci]
- chore: bump version to 0.3.9 and update changelog
- feat: implement comprehensive mock server functionality across all crates
- chore: commit remaining version updates
- fix: enable publishing for mockforge-ui
- fix: enable publishing for mockforge-tunnel
- fix: update all 0.3.7 dependencies to 0.3.8 with path dependencies
- fix: add path dependencies for all workspace crates
- chore: update CHANGELOG date for 0.3.8
- chore: bump version to 0.3.8
- Fix cargo publish issues: add version requirements to dependencies
- chore: update benchmark baseline [skip ci]
- Apply formatting and additional code changes
- Fix compilation errors: update dependencies and adapt to API changes
- fix: remove path from mockforge-pipelines dep in mockforge-collab
- Add mockforge-sdk, mockforge-ui, mockforge-cli to workspace
- fix: add mockforge to restore function targets list
- fix: convert mockforge dev-dependencies to path dependencies
- fix: add mockforge-core to restore list and manually fix dependency
- fix: include mockforge-core in restore list
- fix: restore function now properly handles table-form dependencies without path
- fix: automatically restore dependencies at start of publish
- fix: restore all crate dependencies, not just a few
- fix: only convert dependencies for already-published crates
- fix: correct publish order - publish mockforge-data before mockforge-core
- fix: add mockforge-data as optional dependency in mockforge-core
- chore: bump version to 0.3.6 and update changelog
- chore: update benchmark baseline [skip ci]
- Fix k6 script generation and UI icon embedding issues
- chore: update benchmark baseline [skip ci]
- Add comprehensive test suite and fix build issues
- chore: update benchmark baseline [skip ci]
- docs: add comprehensive performance benchmarks documentation
- chore: update benchmark baseline [skip ci]
- fix: implement real functionality in benchmark tests and fix k8s-operator
- chore: update benchmark baseline [skip ci]
- fix: filter out 'change' directories from benchmark baseline parsing
- chore: update benchmark baseline [skip ci]
- chore: update benchmark baseline [skip ci]
- fix: GitHub Actions workflow cleanup and fixes (#81)
- chore: restore dependencies after publishing all crates
- fix: add mockforge-cli to workspace and add metadata to mockforge-k8s-operator
- fix: add missing crates to workspace (mockforge-sdk, mockforge-http, mockforge-ui, mockforge-k8s-operator)
- fix: add mockforge-world-state to workspace and publishing order before mockforge-http
- fix: add mockforge-route-chaos publishing step before mockforge-http
- fix: add mockforge-route-chaos to dependency targets and publishing order
- fix: add mockforge-route-chaos to workspace and publishing script
- fix: add mockforge-route-chaos to publishing order before mockforge-http
- fix: reduce keywords from 6 to 5 for mockforge-performance
- fix: reduce keywords to 5 for mockforge-performance (crates.io limit)
- fix: add mockforge-performance to publishing order before mockforge-http
- fix: add mockforge-collab to workspace members list
- fix: add mockforge-collab to workspace members
- fix: add missing README.md for mockforge-pipelines
- fix: add mockforge-pipelines to publishing order and dependency targets
- fix: add mockforge-pipelines to workspace and publishing script
- fix: add all missing crates to workspace members
- fix: handle short form dependencies when converting to path
- fix: publish mockforge-template-expansion before mockforge-core
- fix: add mockforge-template-expansion to publishing script
- fix: temporarily convert dependent crates' dependencies to path before publishing
- fix: remove argon2 from mockforge-core during MSRV checks
- fix: exclude mockforge-collab from MSRV checks and remove patch section
- fix: use awk instead of sed for multi-line patch section insertion
- fix: use Cargo patch section to pin base64ct for MSRV
- fix: improve base64ct pinning order in MSRV workflow
- fix: use exact version constraint for base64ct in MSRV workflow
- fix: improve base64ct pinning in MSRV workflow
- fix: pin base64ct to 1.7 for MSRV compatibility
- fix: exclude mockforge-ui from MSRV checks
- fix: add abd and existant to typos config
- fix: exclude FontAwesome and all minified files from spell check
- fix: also remove sysinfo from mockforge-ui during MSRV checks
- fix: exclude elasticlunr.min.js from spell check
- fix: exclude highlight.js from spell check
- fix: disable sysinfo feature during MSRV checks
- fix: sync sysinfo to 0.37, fix resolvable typo, exclude ace.js from spell check
- fix: pin sysinfo to 0.36, fix typos, improve MSRV workaround
- fix: update MSRV to 1.80 and add GraphQL exclusion workaround
- fix: update MSRV from 1.82 to 1.75
- fix: fix GitHub Actions workflow failures
- fix: standardize dependencies and fix all test failures
- Skip CRDs in kubectl validation to avoid server connection
- Fix kubectl validation to prevent server connection attempts
- Fix kubectl validation to skip server connection
- Fix all test failures and resolve dependency conflicts
- Fix k6 metric name validation error (issue #79) (#80)
- Optimize workflows: update deprecated actions and add path filters
- Fix mockforge-smtp version constraint from 0.2.0 to 0.3.3
- Fix Docker build, k8s validation, and spell check issues
- fix: update all mockforge dependency versions to 0.3.3 in mockforge-http
- chore: fix formatting (pre-commit hooks)
- deps(deps): bump opentelemetry_sdk from 0.21.2 to 0.31.0 (#67)
- chore: update benchmark baseline [skip ci]
- deps(deps): bump opentelemetry-semantic-conventions (#66)
- chore: update benchmark baseline [skip ci]
- deps(deps): bump sysinfo from 0.32.1 to 0.37.2 (#60)
- deps(deps): bump wasmparser from 0.239.0 to 0.240.0 (#64)
- deps(deps): bump governor from 0.6.3 to 0.8.1 (#61)
- chore: update benchmark baseline [skip ci]
- deps(deps): bump mail-parser from 0.9.4 to 0.11.1 (#63)
- deps(deps): bump rumqttc from 0.24.0 to 0.25.0 (#65)
- deps(deps): bump ndarray from 0.16.1 to 0.17.1 (#76)
- chore: update benchmark baseline [skip ci]
- ci(deps): bump azure/setup-helm from 3 to 4 (#72)
- ci(deps): bump actions/upload-artifact from 4 to 5 (#71)
- deps(deps): bump image from 0.24.9 to 0.25.9 (#77)
- deps(deps): bump rustls from 0.21.12 to 0.23.35 (#78)
- chore: update benchmark baseline [skip ci]
- Bump all crates to version 0.3.3
- Format code with rustfmt
- Fix k6 script generation with operation IDs containing dots/hyphens
- chore: update benchmark baseline [skip ci]
- perf: optimize template rendering by avoiding unnecessary operations
- chore: update benchmark baseline [skip ci]
- docs: update benchmark documentation with final optimizations
- perf: fix benchmark regressions and optimize measurements
- chore: update benchmark baseline [skip ci]
- Fix Kafka compilation errors and borrow checker issues
- feat: Implement cross-pillar enhancements - World State Engine, MOD, and Performance Mode
- feat(ai-studio): Add API Critique, System Generator, and Behavioral Simulator
- chore: rework UI/UX to be more AI native
- fix: Address pre-commit security vulnerabilities
- feat: Implement Invisible Mock Server experience (DevX Pillar)
- feat(security): implement email, Slack, and webhook notification services
- Refactor template expansion for Send safety
- chore: Restore path dependencies after 0.3.2 publish
- Fix: Complete SQLx query cache for mockforge-collab 0.3.2
- chore: update mockforge dependencies to version 0.3.1 across multiple crates
- fix: improve dependency conversion for optional dependencies and fix publishing order
- fix: update publish script to handle Phase 1 crate dependencies correctly
- feat: add comprehensive integration tests for 0.3.0 features and update changelog
- feat: Complete pillar enhancement gaps - VS Code extension and docs
- feat: Implement pillar tagging system and documentation enhancements
- feat: Implement MockForge AI Studio - Unified AI Copilot
- feat(cloud): Complete Cloud pillar implementation and fix compilation issues
- [DevX] Add JSON Schema support for config validation and IDE autocompletion
- feat: Implement Contract Fitness Functions, Consumer Impact Analysis, and Multi-Protocol Contracts
- feat: Enhance Reality feature with observability, cross-protocol consistency, and time-aware lifecycles
- fix: use proper vosk API by matching on CompleteResult enum
- fix: resolve all compilation errors
- chore: prepare release 0.3.0
- feat: Implement LLM Studio - Natural Language Workspace Creation (0.3.4)
- feat: Complete Behavioral Cloning v1 implementation and refactor architecture
- feat: Implement Drift Budget & GitOps for API Sync + AI Contract Diff
- feat: implement Scenario Studio Visual Editor with React Flow
- feat: implement AI-Native Interface Deepening features
- feat: Implement Time Travel & Snapshots and Frontend X-Ray Mode
- feat(sdk): Add Contract-Backed Types and Scenario-First SDKs to Vue, Svelte, and Angular
- Format code: Apply rustfmt and whitespace cleanup
- Release v0.2.9: Update version, CHANGELOG, and publish all crates to crates.io
- Add registry server improvements, password reset, metrics, and marketplace enhancements
- security: Upgrade wasmtime to 36.0.3 to fix RUSTSEC-2025-0118
- feat: Fix compilation errors and implement comprehensive E2E test suite
- fix: implement custom routes, template expansion, latency injection, and init improvements
- feat: Smart Personas with array generation and relationship inference
- feat: Complete Java and .NET SDK implementations with builder patterns
- fix: update all test files for new function signatures
- fix: resolve all compilation errors across workspace
- Complete Phase 3 security controls implementation
- Add cloud monetization infrastructure and features
- Implement organization management endpoints
- Fix Axum 0.8 route syntax in state_machine_api.rs
- Fix file server route syntax for Axum 0.8 compatibility
- Release v0.2.8: Publish all crates to crates.io
- chore: bump version to 0.2.8
- feat: Complete Generative Schema Mode and achieve 100% roadmap completion
- Implement Smart Personas feature for consistent cross-endpoint data generation
- Add Reality Continuum feature for blending mock and real data sources
- Implement Voice + LLM Interface with STT backends
- Implement complete Deceptive Deploy feature
- Add GraphQL + REST Playground with workspace filtering
- Implement ForgeConnect SDK with full feature set
- Add enhanced scenario marketplace features
- Configure SQLx and integrate mockforge-collab with mockforge-core
- Fix test compilation errors in reality integration and hot-reload tests
- Implement Reality Slider feature with hot-reload support
- Complete latency recording integration and fix WorkspaceConfig reality_level field
- style: Apply rustfmt formatting to Chaos Lab code
- feat: Add Chaos Lab interactive network condition simulation
- Fix test compilation errors in openapi_generator_tests
- Fix all compilation errors for AI Contract Diff feature
- Add WireMock-inspired features: browser proxy mode, git sync, data sources, template library, managed hosting docs, and user management
- Add comprehensive ecosystem and use cases documentation
- Complete configuration and extensibility implementation
- Add advanced behavior and simulation features
- Fix test and benchmark compilation errors
- Complete Scenario State Machines 2.0 with sub-scenario execution
- Implement VBR Engine enhancements: OpenAPI integration, M2M relationships, seeding, ID generation, snapshots
- Add mock-to-real migration pipeline with per-route toggling
- Add Data Scenarios Marketplace feature
- feat: Implement ForgeConnect - Front-End Integrated Mode for browser-based mock creation
- Add MockForge Cloud Graph visualization with real-time updates and export
- Add data personality profiles system for consistent mock data generation
- Add realistic network conditions and chaos lab with interactive UI controls
- Add temporal simulation with CLI commands and scenario support
- Complete MockAI implementation with query params and session recording
- Add Virtual Backend Reality (VBR) engine
- Add multipart form data support and file generation/serving for API mocks
- fix: update mockforge-plugin-sdk to use workspace version
- fix: enable publishing for mockforge-tunnel and add to publish script

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
