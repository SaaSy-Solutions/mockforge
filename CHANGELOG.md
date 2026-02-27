## [0.3.70] - 2026-02-27

### Fixed

- **[Bench]** Remove dead `CUSTOM_HEADERS` JS const from conformance generators (#79)
  - Custom header values are now inlined directly into each request instead of referencing an unused JS constant
  - Eliminates confusing dead code in generated k6 scripts
- **[Bench]** Add `noCookies: true` to k6 options when Cookie header is in custom headers (#79)
  - Prevents k6's automatic cookie jar from duplicating cookies on subsequent requests
  - Fixes duplicate session ID / authentication failures reported by @srikr
- **[Bench]** Fix conformance report file not found after k6 execution (#79)
  - `handleSummary` now writes `conformance-report.json` to an absolute path matching the output directory
  - Previously wrote to a relative path based on k6's CWD, causing the CLI to report "Conformance report not generated"

### Added

- **[Bench]** `--conformance-all-operations` flag for full-endpoint conformance testing (#79)
  - Default mode tests one representative operation per feature check (fast feature-coverage)
  - New flag tests ALL operations with path-qualified check names (e.g., `method:GET:/api/users`)
  - Addresses user confusion about "only 5 endpoints tested"
- **[Bench]** Conformance coverage summary output (#79)
  - After generating conformance tests, prints "Conformance: N operations analyzed, M unique checks generated"
  - When using default mode with fewer checks than operations, shows tip about `--conformance-all-operations`

## [0.3.69] - 2026-02-24

### Fixed

- **[Multi]** Replace 36+ `assert!(true)` placeholder tests with meaningful assertions across 16 files
  - CLI command tests (MQTT, SMTP, governance) now construct and verify command variants
  - Registry server tests use compile-time type checks instead of no-op assertions
  - Integration tests (voice workspace, drift GitOps, behavioral cloning, WebSocket, cross-platform sync) use proper verification patterns
- **[gRPC]** Add `use super::*` to 13 empty `test_module_compiles()` tests so they actually verify module compilation
- **[HTTP]** Fix misleading "placeholder" doc comment on fully-implemented `get_proxy_inspect` handler

## [0.3.57] - 2026-02-14

### Fixed

- **[Bench]** Spec-driven conformance: global security requirement detection (#79)
  - `annotate_security()` now falls back to `spec.security` (root-level) when an operation has no operation-level security defined
  - APIs that only define security globally are now correctly detected
- **[Bench]** Spec-driven conformance: SecurityScheme type resolution (#79)
  - Security schemes are now resolved from `components.securitySchemes` to detect actual type (`HTTP/bearer`, `APIKey`, `HTTP/basic`) instead of relying on name heuristics alone
  - A scheme named "myAuth" that is actually an `apiKey` type is now correctly identified
  - Name-based heuristic retained as fallback for unresolvable schemes
- **[Bench]** Spec-driven conformance: ContentNegotiation detection (#79)
  - `ContentNegotiation` feature is now detected when a response defines multiple content types (e.g., both `application/json` and `application/xml`)
  - Previously only worked in reference mode
- **[Bench]** CLI help text for `--conformance-categories` now includes `response-validation` (#79)

### Added

- **[Bench]** 5 new conformance tests: ResponseValidation with schema check, global security, SecurityScheme resolution, ContentNegotiation detection, single-type negative case (#79)

## [0.3.56] - 2026-02-14

### Added

- **[Bench]** Conformance category filtering (#79)
  - New `--conformance-categories` flag to run only specific conformance categories (e.g., `--conformance-categories "parameters,security"`)
  - Case-insensitive category matching with validation against known categories
- **[Bench]** Spec-driven conformance testing (#79)
  - When `--conformance --spec my-api.json` is provided, analyzes the user's actual OpenAPI spec to detect which features their API exercises
  - Generates conformance tests against real endpoints instead of reference `/conformance/` paths
  - Full `$ref` resolution with cycle detection for parameters, schemas, request bodies, and responses
  - Detects: parameter types, request body formats, schema types/composition/formats/constraints, response codes, security schemes
- **[Bench]** Response schema validation (#79)
  - In spec-driven mode, validates response bodies against OpenAPI response schemas
  - `SchemaValidatorGenerator` produces JavaScript validation expressions from OpenAPI schemas
  - Supports object (required fields, property types), array, string (format regex, enum, length), integer/number (range), boolean validation
  - Wrapped in try-catch for resilient k6 execution
- **[Bench]** SARIF 2.1.0 report output (#79)
  - New `--conformance-report-format sarif` flag outputs conformance results in SARIF 2.1.0 format
  - Compatible with GitHub Code Scanning, VS Code SARIF Viewer, and CI/CD pipelines
  - Maps each conformance feature to a SARIF rule with OpenAPI spec section links
  - Passed features emit `level: "note"`, failed features emit `level: "error"`

## [0.3.55] - 2026-02-14

### Added

- **[Bench]** Per-server stats in multi-target mode (#79)
  - `K6Results` now parses RPS, VUs, and full latency breakdown (min/med/p90/p95/p99/max) from k6 `summary.json`
  - `AggregatedMetrics` includes `total_rps`, `avg_rps`, `total_vus_max`
  - Multi-target reporter shows per-target RPS, VUs, and full latency breakdown
  - `aggregated_summary.json` includes all new metrics in both aggregated and per-target sections
- **[Bench]** Per-target spec support for multi-target mode (#79)
  - Targets file JSON format now supports `"spec"` field for per-target OpenAPI specs
  - Each target can use a different spec file for heterogeneous fan-out
  - Example: `[{"url": "https://server1", "spec": "spec_a.json"}, {"url": "https://server2", "spec": "spec_b.json"}]`
- **[Bench]** OpenAPI 3.0.0 conformance testing (#79)
  - New `--conformance` flag generates and runs comprehensive k6 scripts exercising 47 OpenAPI 3.0.0 features across 10 categories (Parameters, Request Bodies, Schema Types, Composition, String Formats, Constraints, Response Codes, HTTP Methods, Content Negotiation, Security)
  - Reports per-category pass/fail rates with colored terminal output
  - Supports `--conformance-api-key`, `--conformance-basic-auth`, `--conformance-report` for security scheme testing
  - Example: `mockforge bench --conformance --target http://localhost:3000`

## [0.3.54] - 2026-02-13

### Fixed

- **[Bench]** fix(bench): deliver CRS payloads as path injection + form-encoded body (#79)
  - Added `inject_as_path` field to `SecurityPayload` — URI payloads without query params (e.g., CRS 942101: `POST /1234%20OR%201=1`) now replace the request path via `encodeURI()` so WAFs inspect `REQUEST_FILENAME` instead of `ARGS`
  - Added `form_encoded_body` field to `SecurityPayload` — body payloads from CRS tests (e.g., 942432: `var=;;dd foo bar`) now sent as `application/x-www-form-urlencoded` so WAFs parse form data into `ARGS` for character counting
  - Updated `k6_script.hbs` and `k6_crud_flow.hbs` templates to handle both new delivery mechanisms
  - Replaced unreliable `startsWith('/')` URI heuristic in CRUD flow template with explicit `injectAsPath` flag
  - Expected SQLi detection: 46/46 rules (100%), up from 45/46 (97.8%)

## [0.3.53] - 2026-02-13

### Fixed

- **[Bench]** fix(bench): URL-encode URI payloads + strip form keys from body payloads (#79)
  - URI security payloads now wrapped in `encodeURIComponent()` for valid HTTP transport — WAFs decode before inspection (fixes 942101)
  - Form-encoded body payloads now have form key prefix stripped (`var=;;dd foo bar` → `;;dd foo bar`) so WAF ARGS parsing sees the attack payload directly (fixes 942432)
  - Confirmed SQLi detection: 45/46 rules (97.8%), up from 43/46 (93.5%)

## [0.3.52] - 2026-02-12

### Fixed

- **[Bench]** fix(bench): Group multi-part WAFBench payloads + decode body payloads + fix Cookie/CookieJar conflict (#79)
  - Multi-part CRS test cases (URI + headers + body) now grouped by `group_id` and sent together in one HTTP request instead of being split across separate requests (fixes 942290)
  - Body payloads from CRS YAML files are now form-URL-decoded before injection (`%22+WAITFOR+DELAY+%27` → `" WAITFOR DELAY '`) so WAFs see actual SQL patterns in JSON bodies (fixes 942240, 942320, 942432)
  - URI payloads from path-only CRS tests are now URL-decoded and stripped of leading `/` artifact (fixes 942101)
  - Cookie header payloads no longer overridden by empty CookieJar — `secRequestOpts` conditionally skips `jar: new http.CookieJar()` when a security Cookie header is present (fixes 942420, 942421)
  - Added `groupedPayloads` array-of-arrays in generated k6 scripts; `getNextSecurityPayload()` returns arrays of related payloads
  - Template loop applies URI/header/body parts simultaneously per request via `secPayloadGroup`
  - Expected SQLi detection improvement: 37/46 → 44/46 (80.4% → 95.7%)

## [0.3.51] - 2026-02-11

### Fixed

- **[Bench]** fix(bench): Accept all WAFBench CRS payloads without attack-pattern filter (#79)
  - Removed overly strict `attack-pattern` category filter that was silently dropping valid CRS test cases
  - All CRS YAML test cases now loaded regardless of their `attack_type` metadata

## [0.3.50] - 2026-02-10

### Fixed

- **[Bench]** fix(bench): Use per-request CookieJar instead of shared EMPTY_JAR (#79)
  - Each HTTP request now creates its own `new http.CookieJar()` instead of sharing a global empty jar
  - Prevents cookie cross-contamination between requests in security testing

## [0.3.49] - 2026-02-09

### Fixed

- **[Bench]** fix(bench): Send raw security payloads + use dedicated empty cookie jar (#79)
  - Security payloads now sent as raw strings without additional encoding
  - Dedicated empty CookieJar per request prevents k6's default cookie accumulation

## [0.3.48] - 2026-02-08

### Fixed

- **[Bench]** fix(bench): Cycle security payloads per-operation + clear cookies in API2 tests (#79)
  - Security payloads now cycle per-operation block (each API endpoint gets a different payload)
  - Previously all operations in one VU iteration used the same payload
  - OWASP API2 (Broken Auth) tests now properly clear cookies between requests

## [0.3.47] - 2026-02-06

### Added

- **[DevX]** chore: Add Claude Code setup (CLAUDE.md, agents, skills, hooks, hookify)
  - Project-specific Claude Code configuration with rules, agents, and skills
  - Custom skills for verification, template checking, code review, and bench review
  - Hookify rules engine for behavioral guardrails

### Fixed

- **[Bench]** fix(bench): Security payloads now injected + cookie dedup in all templates (#79)
  - Security payloads now properly injected in both k6_script.hbs and k6_crud_flow.hbs templates
  - Cookie deduplication applied to all HTTP request paths in both templates
  - Comprehensive test suite added for issue #79 security pipeline

- **[Registry]** fix(registry): Add RBAC permission system with Display, AdminAll bypass, and PermissionChecker
  - New RBAC permission model with role-based access control
  - AdminAll role bypasses all permission checks
  - PermissionChecker trait for consistent authorization across endpoints

## [0.3.46] - 2026-01-30

### Fixed

- **[Bench]** fix(bench): WAFBench payloads now distributed across VUs for better coverage (#79)
  - Changed payload cycling to use VU-based offset: `(__VU - 1) % payloads.length`
  - Previously all 50 VUs started at index 0 and cycled through same sequence
  - Now each VU starts at a different payload, maximizing attack coverage in shorter test runs
  - With 50 VUs and 30 payloads, all payloads are tested from the start

- **[Bench]** fix(bench): OWASP API tests now include custom headers in all requests (#79)
  - Added `CUSTOM_HEADERS` to API8 verbose error test (malformed JSON body test)
  - Added `CUSTOM_HEADERS` to API9 discovery paths test
  - Added `CUSTOM_HEADERS` to API9 API versions test
  - Fixes auth failures when using `--headers "Cookie:..."` with OWASP testing

## [0.3.43] - 2026-01-16

### Fixed

- **[Bench]** fix(bench): Security payloads now actually applied to requests in k6 scripts (#79)
  - Updated k6_script.hbs template to call `getNextSecurityPayload()` and `applySecurityPayload()`
  - Previously, security payload functions were defined but never called in generated scripts
  - Security payloads now properly injected into request bodies for POST/PUT/PATCH
  - Header-based payloads now properly injected into request headers

## [0.3.42] - 2026-01-15

### Fixed

- **[Bench]** fix(bench): XSS payloads now inject into ALL string fields, not just the first one (#79)
  - Removed `break` statement from `applySecurityPayload()` loop in security_payloads.rs
  - Ensures WAF can detect payloads regardless of which field it scans
- **[Bench]** fix(bench): Added `jar: null` to remaining OWASP HTTP calls to prevent cookie duplication (#79)
  - Fixed testBrokenAuth empty token test
  - Fixed testMisconfiguration verbose error test
  - Fixed testInventory discovery paths and API versions checks
- **[CLI]** fix(cli): Fixed format string compilation error in plugin_commands.rs (#79)
  - Escaped all braces (`{` → `{{`, `}` → `}}`) inside `format!` macro for auth plugin template
  - Fixes "invalid format string: expected `}`, found `r`" compilation error

## [0.3.39] - 2026-01-14

### Fixed

- **[Bench]** fix(bench): WAFBench XSS attacks now properly injected into request body (#79)
  - Removed location check from `applySecurityPayload()` - ALL payloads now injected into body for POST/PUT
  - WAFBench payloads correctly pass location info (uri/header/body) to k6 scripts
  - Header payloads include header name for proper injection into specified headers
- **[Bench]** fix(bench): Cookie header duplication in OWASP and security tests (#79)
  - Added `jar: null` to all HTTP request params to disable k6's automatic cookie jar
  - Prevents duplicate cookies when user provides Cookie header via `--headers` flag
  - Applied to k6_script.hbs, k6_crud_flow.hbs, and OWASP generator

## [0.3.38] - 2026-01-13

### Fixed

- **[Bench]** fix(bench): pass custom headers from `--headers` flag to OWASP tests (#79)
  - Cookie and other custom headers are now included in all OWASP request helpers
  - Fixes issue where `avi-sessionid=None` was being sent instead of actual cookie values
- **[Bench]** fix(bench): WAFBench loader now handles single YAML file paths (#79)
  - Previously only directories or glob patterns were supported
  - Single file paths like `/path/to/941100.yaml` now work correctly
- **[Bench]** Verified CRS v3.3 format compatibility with full CoreRuleSet test suite
  - Tested with 175 files, 1512 payloads (692 XSS, 505 SQLi, 304 Command Injection, 11 Path Traversal)

## [0.3.37] - 2026-01-12

### Added

- **[Bench]** feat(bench): add WAFBench cycle-all mode (`--wafbench-cycle-all`) to test all payloads sequentially (#79)
- **[Bench]** feat(bench): add `--owasp-iterations` parameter to control OWASP test iterations per VU (#79)
- **[Bench]** feat(bench): OWASP tests now respect `--vus` parameter for concurrent testing (#79)

### Fixed

- **[Bench]** fix(bench): WAFBench payloads now properly injected in standard bench mode (not just CRUD flow)
- **[Bench]** fix(bench): OWASP APIs now use random UUIDs per request instead of static IDs for BOLA testing (#79)
- **[Bench]** fix(bench): OWASP auth tokens with special characters (quotes, backslashes) now properly escaped (#79)
- **[Bench]** fix(bench): prevent Handlebars double-escaping of pre-escaped JavaScript values
- **[Bench]** fix(bench): WAFBench security payloads now integrated into CRUD flow requests (#79)
- **[Bench]** fix(owasp): use `http.del()` instead of `http.delete()` for k6 compatibility (#79)
- **[Bench]** fix(owasp): add `--base-path` support for OWASP API testing (#79)
- **[Bench]** fix(bench): remove undefined `totalRequestCount` variable reference
- **[Bench]** fix(bench): support CRS v3.3 WAFBench format and pass `--insecure` to OWASP tests

## [0.3.33] - 2026-01-10

### Fixed

- **[Bench]** fix(bench): multiple fixes for OWASP and WAFBench testing
  - Support CRS v3.3 format in WAFBench parser
  - Pass `--insecure` flag to OWASP tests for self-signed certificates

## [0.3.31] - 2026-01-08

### Fixed

- **[Bench]** fix(bench): fix extracted value substitution in CRUD flows
- **[Bench]** fix(bench): OWASP k6 configuration improvements

## [0.3.30] - 2026-01-07

### Added

- **[Bench]** feat(bench): add `merge_body` support for CRUD flows - merge extracted values with request body
- **[Bench]** feat(bench): add `inject_attacks` data model for security testing in CRUD flows

## [0.3.28] - 2026-01-06

### Added

- **[Bench]** feat(bench): add nested path extraction for CRUD flows (e.g., `results[0].id`)
- **[Bench]** feat(bench): add filter extraction for CRUD flows (e.g., `results[?name=='test'].id`)

## [0.3.27] - 2026-01-05

### Added

- **[Bench]** feat(bench): add full body extraction for CRUD flows
- **[Bench]** feat(bench): add key filtering for extracted values

## [0.3.26] - 2026-01-04

### Added

- **[Bench]** feat(bench): add aliased extraction for CRUD flow value chaining
  - Extract values with aliases (e.g., `id as poolId`) for use in subsequent requests

## [0.3.24] - 2026-01-03

### Fixed

- **[Bench]** fix(bench): use correct variable name in CRUD flow extracted value replacement

## [0.3.22] - 2026-01-02

### Added

- **[Bench]** feat(bench): add OWASP API Security Top 10 testing mode (#79)
  - Test for BOLA (API1), Broken Auth (API2), Mass Assignment (API3), Resource Consumption (API4)
  - Test for Function Auth (API5), SSRF (API7), Misconfiguration (API8), Inventory (API9), Unsafe Consumption (API10)
  - Configurable test categories with `--owasp-categories`
  - Support for auth tokens with `--owasp-auth-token`
  - SARIF and JSON report formats

### Changed

- **[DevX]** chore: include UI dist files for publishing to crates.io

## [0.3.21] - 2025-12-31

### Fixed

- **[DevX]** fix(bench): use custom flow config and fix sequential mode path matching - enables cross-resource dependency chains
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

## [0.3.20] - 2025-12-31

### Fixed

- **[Bench] Dynamic placeholder expansion in CRUD flow params file bodies** (#79): Fixed `${__VU}`, `${__ITER}`, and other dynamic placeholders not being expanded when used in request body content from params files
  - Previously, placeholders like `"name": "HTTP-WAAP-vsvip-${__VU}-${__ITER}"` were sent literally to the API
  - Now properly converted to k6 template literals for runtime evaluation
  - Supports all dynamic placeholders: `${__VU}`, `${__ITER}`, `${__TIMESTAMP}`, `${__UUID}`, `${__RANDOM}`, `${__COUNTER}`, `${__DATE}`, `${__VU_ITER}`

## [0.3.19] - 2025-12-30

### Added

- **[DevX] API base path support for bench command** (#79): New `--base-path` option to prepend a path prefix to all API endpoints in generated load tests
  - Automatically extracts base path from OpenAPI spec's `servers` URL (e.g., `https://api.example.com/api/v1` → `/api/v1`)
  - CLI option takes priority over spec's base path for explicit control
  - Use `--base-path ""` to disable base path even if spec defines one
  - Works with both standard k6 scripts and CRUD flow mode
  - Example usage:
    ```bash
    # Auto-detect from spec's servers URL
    mockforge bench --spec api.yaml --target http://localhost:8080 --crud-flow

    # Explicitly set base path
    mockforge bench --spec api.yaml --target http://localhost:8080 --base-path /api

    # Disable base path
    mockforge bench --spec api.yaml --target http://localhost:8080 --base-path ""
    ```

## [0.3.18] - 2025-12-29

### Fixed

- **[Collab] SQLx offline mode for crates.io installation** (#79): Fixed compilation errors when installing `mockforge-collab` from crates.io
  - Added `.sqlx` query cache directory with 51 precompiled query metadata files
  - The `build.rs` now automatically enables `SQLX_OFFLINE=true` when query cache is present
  - Users no longer need `DATABASE_URL` or to run `cargo sqlx prepare` to install the crate
  - Resolves "set DATABASE_URL to use query macros online" compilation errors

## [0.3.17] - 2025-12-28

### Added

- **[DevX] WAFBench YAML integration for security testing**: New `--wafbench-dir` flag to import Microsoft WAFBench CRS (Core Rule Set) attack patterns
  - Parse WAFBench YAML test files from the [WAFBench project](https://github.com/microsoft/WAFBench)
  - Support glob patterns for loading specific rule categories (e.g., `REQUEST-941-*` for XSS, `REQUEST-942-*` for SQLi)
  - Extract attack payloads from URI parameters, headers, and request bodies
  - Automatic CRS rule ID parsing from test metadata (e.g., `941100` for XSS attacks)
  - Integrate WAFBench payloads with existing security testing framework
  - Example usage:
    ```bash
    mockforge bench spec.yaml --wafbench-dir ./wafbench/REQUEST-941-*  # XSS rules
    mockforge bench spec.yaml --wafbench-dir ./wafbench/**/*.yaml      # All rules
    ```

- **[DevX] Per-URI control mode for data-driven testing** (#79): New `--per-uri-control` flag for CSV/JSON data files that allows each row to specify HTTP method, URI, body, query params, headers, attack type, and expected status code
  - Enables fine-grained control over test requests directly from data files
  - Supports security testing per-URI with `attack_type` column
  - Automatic status validation with `expected_status` column
  - Example CSV format:
    ```csv
    method,uri,body,query_params,headers,attack_type,expected_status
    GET,/virtualservice,,include_name=true,,,200
    POST,/virtualservice,"{""name"":""test""}",,,sqli,201
    ```

- **[Protocol] AMQP TLS support**: Full TLS/SSL support for AMQP broker with configurable certificates
- **[Protocol] MQTT protocol improvements**: Enhanced MQTT server with TLS, session management, and metrics
- **[Protocol] gRPC dynamic service improvements**: Better dynamic proto loading and error handling
- **[Registry] Security enhancements**: CSRF protection, request ID middleware, trusted proxy support, token revocation
- **[UI] Frontend improvements**: Type safety fixes, mobile layout improvements, search/filter functionality

### Changed

- Comprehensive dependency updates across workspace crates

### Fixed

- **[DevX] CRUD flow params file integration** (#79): Fixed `--params-file` not being applied in CRUD flow mode
  - Body configurations from params file are now correctly applied to POST/PUT/PATCH operations in `--crud-flow` mode
  - Fixed body serialization issue that caused "ReferenceError: object is not defined" error in generated k6 scripts
  - Body is now properly serialized as a JSON string for the Handlebars template
- **[Core] Race conditions and flaky tests**: Resolved timing issues across test suite
- **[Core] Panic-prone unwrap calls**: Replaced with safe error handling throughout codebase

## [0.3.16] - 2025-12-27

### Added

- Version bump with dependency updates

### Fixed

- **[Test] Flaky test fixes**: Resolved race conditions and timing issues in integration tests
- **[Core] Safe error handling**: Replaced panic-prone `.unwrap()` calls with proper error handling

## [0.3.15] - 2025-12-26

### Added

- **[DevX] Multi-spec support for bench command**: The `mockforge bench` command now supports loading and merging multiple OpenAPI specifications
  - Multiple `--spec` flags: `mockforge bench --spec pools.yaml --spec vs.yaml --target https://api.com`
  - Directory discovery with `--spec-dir`: `mockforge bench --spec-dir ./specs/ --target https://api.com`
  - Conflict resolution strategies with `--merge-conflicts`: `error` (default), `first`, `last`
  - Spec mode selection with `--spec-mode`: `merge` (default) combines all specs, `sequential` runs specs in dependency order
  - Sequential execution mode with per-spec output directories and results
  - Leverages existing multi-spec infrastructure from mockforge-core
- **[DevX] Cross-spec dependency detection**: New `spec_dependencies` module for handling dependencies between specs
  - Automatic detection of dependencies from field naming patterns (`pool_ref`, `pool_id`, `poolId`, etc.)
  - Schema registry for cross-referencing schemas across multiple specs
  - Topological sorting for correct execution order
  - Manual dependency configuration via `--dependency-config` (YAML/JSON)
  - Support for value extraction and injection between spec groups

### Changed

- `BenchCommand.spec` field changed from `PathBuf` to `Vec<PathBuf>` to support multiple specs
- `SpecParser` now includes `from_spec()` method for pre-loaded OpenAPI specs
- Added `dependency_config` field to `BenchCommand` for cross-spec value passing configuration

### Fixed

- Nothing yet.

## [0.3.14] - 2025-12-26

### Added

- Version bump to 0.3.14

### Changed

- Nothing yet.

### Fixed

- Nothing yet.

## [0.3.13] - 2025-12-24

### Fixed

- **[DevX] k6 CRUD flow metric name sanitization** (#79 follow-up): Fixed invalid k6 metric names in CRUD flow scripts when flow names contain dots or special characters
  - CRUD flow names are now sanitized for use as k6 metric names (e.g., `plans.list` → `plans_list`)
  - Original flow names preserved in comments and group names for readability
  - Made `sanitize_js_identifier` function public for reuse across k6 generators
  - Added script validation to CRUD flow generation for defense in depth

## [0.3.12] - 2025-12-23

### Changed

- **[DevX] Dependency updates**: Version alignment and dependency updates across all workspace crates

## [0.3.11] - 2025-12-19

### Added

- **[DevX] Custom benchmark parameters**: Added `--params-file` option to `mockforge bench` command for loading custom parameter values from a file

  **Why it matters**: Allows users to define reusable parameter configurations for benchmark runs, making it easier to test different scenarios without modifying command-line arguments each time.

## [0.3.10] - 2025-12-18

### Fixed

- **[DevX] k6 benchmark script generation fixes**: Resolved multiple issues with generated k6 scripts
  - Fixed `--insecure` flag handling by moving `insecureSkipTLSVerify` to global k6 options
  - Fixed `textSummary` import and usage in generated scripts
  - Fixed query parameter encoding in benchmark requests

## [0.3.9] - 2025-12-17

### Added

- **[Reality] Comprehensive Mock Server Implementation**: Full implementation across all protocol crates
  - **mockforge-amqp**: Complete AMQP 0-9-1 broker with exchanges, queues, bindings, messages, protocol handling, fixtures, and spec registry
  - **mockforge-kafka**: Full Kafka broker with consumer groups, partitions, topics, metrics, and protocol handling
  - **mockforge-mqtt**: Complete MQTT broker with QoS levels, topic subscriptions, and retained messages
  - **mockforge-ftp**: Virtual filesystem, spec registry, and fixture support
  - **mockforge-smtp**: Email server with fixtures and spec registry
  - **mockforge-tcp**: TCP server with fixtures and protocol support
  - **mockforge-grpc**: Dynamic proto parser, service generator, reflection, and metrics
  - **mockforge-graphql**: Full handler implementations

- **[DevX] Enhanced CLI Commands**: New commands for all protocols and features
  - AMQP, Kafka, MQTT, FTP, SMTP protocol commands
  - Blueprint, cloud, deploy, dev-setup, governance commands
  - Logs, progress, recorder, scenario, snapshot commands
  - Time manipulation, VBR, voice, wizard, and workspace commands
  - AI-powered mock generation commands

- **[Reality] Virtual Backend Repository (VBR)**: Complete data management system
  - API generator, entity management, constraints, and validation
  - Database integration with migrations and schema management
  - Session handling, snapshots, and mutation rules
  - ID generation strategies and scheduling

- **[Reality] World State Engine**: Coherent world simulation
  - State engine with model and query support
  - Entity relationships and lifecycle management

- **[AI] Enhanced AI Capabilities**: AI-powered mock generation
  - RAG-based AI response generator
  - AI event generator for WebSocket scenarios
  - Behavioral cloning with scenario types

- **[Cloud] Collaboration Features**: Team collaboration support
  - Backup, merge, and promotion workflows
  - Multi-environment configuration
  - Client SDK improvements

- **[DevX] Observability & Analytics**: Enhanced monitoring
  - Pillar usage tracking and analytics queries
  - Metrics middleware and coverage tracking
  - Latency metrics and performance monitoring

- **[Contracts] Chaos Engineering**: Resilience testing capabilities
  - Failure designer and incident replay
  - Chaos API with configurable fault injection
  - Route-level chaos with latency distributions

- **[DevX] Plugin System Enhancements**: Extended plugin capabilities
  - Backend generator and datasource support
  - Runtime adapter improvements
  - SDK builders and testing utilities

- **[Cloud] Registry Server**: Complete registry implementation
  - Authentication, authorization, and RBAC
  - Redis caching, email notifications
  - Organization and subscription models
  - API token management and audit logging

- **[DevX] UI Server**: Dashboard and admin features
  - Admin handlers for workspace management
  - Chain visualization and coverage metrics
  - Failure analysis and promotion workflows
  - Graph visualization and health monitoring

## [0.3.8] - 2025-01-27

### Fixed

- **[DevX] Compilation errors resolved**: Fixed all compilation errors across the workspace
  - Updated `axum-server` from 0.6 to 0.8 with `tls-rustls-no-provider` feature
  - Updated `rustls` from 0.21 to 0.23, `rustls-pemfile` from 1.0 to 2.0, `tokio-rustls` from 0.24 to 0.26
  - Adapted TLS code to rustls 0.23 API (CertificateDer, PrivateKeyDer, WebPkiClientVerifier)
  - Fixed multi_spec module: properly exported and resolved compilation errors
  - Fixed handle_serve function calls: added missing parameters and fixed type mismatches
  - Fixed borrow checker issues in multi_spec merging logic
  - Added missing documentation for enum variants and struct fields
  - Fixed various type mismatches and iteration patterns

- **[DevX] Cargo publish readiness**: Fixed all dependency version requirements for crates.io publishing
  - Added version requirements to all path dependencies in mockforge-cli, mockforge-chaos, mockforge-http, mockforge-route-chaos, mockforge-vbr
  - Set `publish = false` for desktop-app and tests packages (not meant for crates.io)
  - All crates now pass `cargo publish --dry-run` validation

## [0.3.6] - 2025-11-25

### Fixed

- **[DevX] k6 script generation with operation IDs containing dots/hyphens** (#79)
  - Fixed "Unexpected token ." error when OpenAPI operation IDs contain dots (e.g., `plans.create`) or hyphens (e.g., `plans.update-pricing-schemes`)
  - Changed `is_alphanumeric()` to `is_ascii_alphanumeric()` in JavaScript identifier sanitization to ensure ASCII-only identifiers
  - All operations are now properly included in generated k6 scripts with valid JavaScript identifiers
  - Added comprehensive tests including integration test with full billing subscriptions spec

- **[DevX] UI icon embedding for published crates**
  - Fixed build failures when installing `mockforge-cli` from crates.io due to missing icon files
  - Updated `build.rs` to read icon files at build time and embed them as byte array literals
  - Replaced `include_bytes!` with `CARGO_MANIFEST_DIR` approach that failed in published crates
  - Icons are now properly embedded and work both in development and when installing from crates.io

## [0.3.0] - 2025-11-17

### Added

- **[DevX] Pillars & Tagged Changelog**: Complete pillar system implementation with documentation and tooling
  - Defined five foundational pillars: [Reality], [Contracts], [DevX], [Cloud], [AI]
  - Added comprehensive PILLARS.md documentation with feature mappings
  - Implemented CI validation for pillar tags in changelog entries
  - Added pillar tagging instructions to release tooling
  - Updated README and getting-started guide with pillars section

  **Why it matters**: Clear product story spine that makes it obvious what each release invests in. Pillar tags help users understand product direction and find features relevant to their needs.

- **[Reality] Smart Personas & Reality Continuum v2**: Complete persona graph and lifecycle system
  - Persona graphs with relationship linking across entities
  - Lifecycle states (NewSignup, Active, PowerUser, ChurnRisk, Churned, etc.)
  - Reality Continuum integration with field-level and entity-level mixing
  - Fidelity score calculation and API endpoint
  - Comprehensive PERSONAS.md documentation

  **Why it matters**: Upgrade from "random-but-consistent fake data" to "coherent world simulation." Personas maintain relationships across endpoints, and fidelity scores quantify how real your mock environment is.

- **[Contracts] Drift Budget & GitOps for API Sync**: Complete drift management system
  - Hierarchical drift budget configuration (global, workspace, service, endpoint)
  - Breaking change detection and classification
  - Incident management with webhook integration
  - GitOps PR generation for contract updates
  - Comprehensive DRIFT_BUDGETS.md documentation

  **Why it matters**: Make MockForge the "drift nerve center" for contracts. Define acceptable drift, get alerts when budgets are exceeded, and automatically generate PRs to update contracts and fixtures.

- **[Reality] Behavioral Cloning v1**: Multi-step flow recording and replay
  - Flow recording with request/response capture and timing
  - Flow viewer with timeline visualization
  - Scenario replay engine with strict/flex modes
  - Scenario storage and export/import (YAML/JSON)
  - Comprehensive BEHAVIORAL_CLONING.md documentation

  **Why it matters**: Move from endpoint-level mocks to journey-level simulations. Record realistic flows from real systems and replay them as named scenarios for comprehensive testing.

- **[AI][DevX] LLM/Voice Interface for Workspace Creation**: Natural language to complete workspace
  - Natural language workspace creation from descriptions
  - Automatic persona and relationship generation
  - Behavioral scenario generation (happy path, failure, slow path)
  - Reality continuum and drift budget configuration from NL
  - Voice and text input support
  - Comprehensive LLM Studio documentation

  **Why it matters**: The golden path: "Describe the system in natural language → MockForge builds a realistic mock backend with personas, behaviors, and reality level config." No manual configuration required.

- **[DevX] Comprehensive Integration Test Coverage**: Complete test suite for all 0.3.0 features
  - Smart Personas v2 integration tests (15 tests covering persona graphs, lifecycle states, fidelity scores)
  - Drift Budget integration tests (14 tests covering budget hierarchy, breaking change detection, incident management)
  - Drift GitOps integration tests (16 tests covering PR generation, OpenAPI/fixture updates, GitOps configuration)
  - Behavioral Cloning integration tests (15 tests covering flow recording, scenario replay, strict/flex modes)
  - Voice/LLM Workspace Creation integration tests (16 tests covering command parsing, workspace building, NL to workspace flow)
  - All tests passing with 100% success rate (76 total integration tests)

  **Why it matters**: Production-ready features require production-ready tests. Comprehensive integration test coverage ensures reliability, prevents regressions, and provides confidence for users adopting these features.

### Changed

- Changelog entries now require pillar tags for all major features
- Release process includes automated pillar tag validation
- Documentation structure updated to highlight pillars

### Fixed

- Nothing yet.

### Security

- Nothing yet.

## [0.2.9] - 2025-11-14

### Added

- **[Cloud] Registry server improvements** with password reset functionality

  **Why it matters**: Enable seamless team collaboration with secure registry access—teams can share and discover mock scenarios without friction, and password reset keeps workflows moving when credentials are lost.

- **[Cloud] Enhanced metrics and marketplace features**
- **[DevX] Comprehensive E2E test suite**
- **[DevX] Custom routes implementation**
- **[Reality] Template expansion improvements**
- **[Reality] Latency injection enhancements**
- **[Reality] Smart Personas** with array generation and relationship inference

  **Why it matters**: Generate realistic, interconnected mock data automatically—arrays that make sense, relationships that stay consistent across endpoints, and personas that feel like real users without manual configuration.

- **[DevX] Complete Java and .NET SDK implementations** with builder patterns

  **Why it matters**: Bring MockForge to enterprise teams using Java and .NET—no more language barriers, no more custom integration work. Your entire stack can use the same mock infrastructure.

- **[Cloud] Cloud monetization infrastructure and features**

  **Why it matters**: Enable sustainable platform growth with flexible pricing models—teams can scale from free tier to enterprise without friction, and the platform can grow while serving developers.

- **[Cloud] Organization management endpoints**

  **Why it matters**: Scale from solo developer to enterprise team—manage users, permissions, and resources at the org level, not just individual accounts. Real teams need real organization tools.

- **[Cloud] Security controls implementation** (Phase 3)

  **Why it matters**: Protect production deployments with enterprise-grade security—fine-grained access controls, audit trails, and compliance features that let you trust MockForge with sensitive data and critical workflows.

### Changed

- **[DevX] Upgraded wasmtime to 36.0.3** to fix RUSTSEC-2025-0118
- **[DevX] Fixed Axum 0.8 route syntax compatibility** across multiple modules
- **[DevX] Updated all test files** for new function signatures

### Fixed

- **[DevX] Fixed compilation errors** across workspace
- **[DevX] Fixed Axum 0.8 route syntax** in state_machine_api.rs
- **[DevX] Fixed file server route syntax** for Axum 0.8 compatibility
- **[DevX] Resolved all compilation errors** for comprehensive test coverage

### Security

- **[DevX] Upgraded wasmtime to 36.0.3** to address RUSTSEC-2025-0118
- **[Cloud] Completed Phase 3 security controls implementation**

## [0.2.8] - 2025-11-10

### Added

- **[Reality] Generative Schema Mode**: Complete implementation of generative schema mode for dynamic mock data generation

  **Why it matters**: Spin up a believable API even when the backend doesn't exist yet—no sample DB or seed data required.

- **[Reality] Smart Personas**: Feature for consistent cross-endpoint data generation using persona-based templates

- **[Reality] Reality Continuum**: Feature for blending mock and real data sources with configurable reality levels

  **Why it matters**: Turn the dial between deterministic mock and noisy production-like chaos without changing your client code.

- **[Reality] Reality Slider**: Hot-reload support for reality level adjustments

  **Why it matters**: Adjust reality levels on the fly during development and testing without restarting the server.

- **[Reality] Chaos Lab**: Interactive network condition simulation tool

  **Why it matters**: Test how your application handles real-world network conditions like latency spikes, packet loss, and connection failures.

- **[Contracts] AI Contract Diff**: Feature for comparing and diffing API contracts

  **Why it matters**: Automatically detect and visualize API contract changes to catch breaking changes before they reach production.

- **[DevX] Voice + LLM Interface**: Voice interface implementation with Speech-to-Text (STT) backend support

- **[Reality] Deceptive Deploy**: Complete deceptive deploy feature for advanced testing scenarios

- **[DevX] GraphQL + REST Playground**: Interactive playground with workspace filtering capabilities

- **[DevX] ForgeConnect SDK**: Complete SDK implementation with full feature set

- **[Cloud] Enhanced Scenario Marketplace**: Improved scenario marketplace with additional features

- **[DevX] WireMock-Inspired Features**: Browser proxy mode, git sync, data sources, template library, managed hosting documentation, and user management

- **[DevX] Ecosystem Documentation**: Comprehensive ecosystem and use cases documentation

- **[DevX] Configuration Extensibility**: Complete configuration and extensibility implementation

- **[Reality] Advanced Behavior Simulation**: Enhanced behavior and simulation features

### Changed

- **[DevX] SQLx Integration**: Configured SQLx and integrated mockforge-collab with mockforge-core
- **[Reality] Latency Recording**: Completed latency recording integration with WorkspaceConfig reality_level field support

### Fixed

- **[DevX] Fixed test compilation errors** in reality integration and hot-reload tests
- **[DevX] Fixed test compilation errors** in openapi_generator_tests
- **[Contracts][DevX] Fixed all compilation errors** for AI Contract Diff feature
- **[DevX] Applied rustfmt formatting** to Chaos Lab code

### Security

- Nothing yet.

## [0.2.7] - 2025-11-05

### Added

- **[Contracts] Automatic API Sync & Change Detection**: Implemented periodic polling and automatic sync for detecting upstream API changes

  **Why it matters**: Keep your mocks in sync with real APIs automatically—catch breaking changes before they break your tests.

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

- **[Reality] TCP Protocol Support**: Added raw TCP server mocking support via new `mockforge-tcp` crate

  **Why it matters**: Mock any protocol that runs over TCP—not just HTTP. Perfect for testing database clients, custom protocols, and legacy systems.

  - Raw TCP connection handling with fixture-based matching
  - Echo mode for testing TCP clients
  - TLS/SSL support for encrypted connections
  - Delimiter-based message framing (optional)
  - Configurable buffer sizes and connection limits
  - CLI flag `--tcp-port` for custom TCP server port
  - Configuration via `config.tcp` in YAML/JSON config files

- **[Reality] Response Selection Modes**: Added support for sequential (round-robin) and random response selection when multiple examples are available
  - Sequential mode: Cycles through available examples in order (round-robin)
  - Random mode: Randomly selects from available examples
  - Weighted random mode: Random selection with custom weights per example
  - Configuration via `x-mockforge-response-selection` OpenAPI extension
  - Environment variable support: `MOCKFORGE_RESPONSE_SELECTION_MODE` (global) and `MOCKFORGE_RESPONSE_SELECTION_<OPERATION_ID>` (per-operation)
  - State tracking for sequential mode ensures round-robin behavior across requests

- **[Reality] Webhook HTTP Execution**: Implemented actual HTTP request execution in chaos orchestration hooks
  - `HookAction::HttpRequest` now executes real outbound HTTP requests (previously only logged)
  - Supports GET, POST, PUT, DELETE, PATCH methods
  - Configurable request body and headers
  - Error handling and logging for webhook failures
  - Fire-and-forget execution (failures don't block orchestration)

- **[DevX] CRUD & Webhook Documentation**: Added comprehensive documentation guides
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

## [0.2.6] - 2025-11-04

### Added

- **[DevX] TLS/HTTPS and mTLS Support**: Added TLS/HTTPS and mutual TLS (mTLS) support for HTTP server
  - Configurable TLS certificate and key paths
  - Client certificate authentication support
  - Secure connection handling for production deployments

- **[DevX] Built-in Tunneling Service**: Added built-in tunneling service for exposing local servers via public URLs
  - Automatic tunnel creation for local development
  - Public URL generation for testing and demos
  - Integration with popular tunneling services

- **[DevX] SDK Implementation**: Completed Phase 1 & 2 of SDK implementation
  - Comprehensive documentation and examples
  - Production-ready client generators

### Changed

- **[DevX] Version Bumps**: Updated all workspace crates from 0.2.5 to 0.2.6
  - Updated all dependency versions across the workspace
  - Fixed version mismatches in mockforge-ui and mockforge-plugin-loader

- **[DevX] Publishing Improvements**: Enhanced crate publishing process
  - Added mockforge-tcp and mockforge-test to publish script
  - Enabled publishing for mockforge-test crate
  - Fixed mockforge-tcp to remove README requirement

### Fixed

- **[DevX] Documentation**: Fixed missing module-level documentation in test files
  - Added comprehensive module documentation to all test modules
  - Improved code documentation consistency

- **[DevX] Axum Compatibility**: Fixed Axum 0.8 compatibility issues in proxy server module
  - Updated proxy server to work with latest Axum version
  - Resolved breaking changes from Axum upgrade

- **[Reality] MQTT Error Types**: Fixed MQTT publish handlers error types to be Send + Sync
  - Updated error types for proper async/await compatibility
  - Ensured thread-safety in MQTT handlers

## [0.2.5] - 2025-01-27

### Added

- **[DevX] OAuth2 Flow Support**: Complete OAuth2 implementation with all standard flows
  - Authorization Code flow with PKCE (RFC 7636 compliant, SHA256 hash)
  - Client Credentials flow for server-side applications
  - Password flow for trusted clients
  - Implicit flow support
  - Automatic token refresh and expiration management
  - State parameter for CSRF protection
  - PKCE code verifier/challenge generation helpers
  - Token storage with expiration tracking (localStorage)

- **[DevX] Enterprise Error Handling**: Structured error handling for generated clients
  - `ApiError` class with status codes, statusText, and error body
  - `RequiredError` class for missing required fields
  - Helper methods: `isClientError()`, `isServerError()`, `getErrorDetails()`, `getVerboseMessage()`
  - Optional verbose error messages with detailed validation information

- **[Contracts] Request/Response Validation**: Built-in validation support
  - Required field validation before sending requests
  - Basic response structure validation (type checking, object validation)
  - Configurable via `validateRequests` flag
  - Detailed validation error messages

- **[DevX] Request/Response Interceptors**: Custom request/response/error transformation
  - Request interceptor: Modify requests before sending
  - Response interceptor: Transform responses after receiving
  - Error interceptor: Global error handling
  - Support for async interceptors

- **[DevX] Enhanced Authentication**: Multiple authentication methods
  - Bearer token (static or dynamic function)
  - API key authentication (static or dynamic)
  - Basic authentication (username/password)
  - OAuth2 (all flows, takes priority over other methods)

- **[DevX] PKCE Helper Functions**: Exported utilities for PKCE implementation
  - `generatePKCECodeVerifier()`: Generate cryptographically random code verifier
  - `generatePKCECodeChallenge()`: Generate SHA256 code challenge from verifier

- **[DevX] Security Best Practices**: Comprehensive security warnings and guidance
  - Client secret warnings for browser-based applications
  - XSS vulnerability warnings for localStorage token storage
  - CSRF protection via state parameter validation
  - Token expiration checking
  - Security documentation in generated README

- **[DevX] Request Timeout Handling**: Configurable request timeouts
  - Default 30-second timeout (configurable)
  - AbortController-based timeout implementation
  - Proper timeout error handling

- **[DevX] React Query Integration Documentation**: Comprehensive examples for @tanstack/react-query integration

### Changed

- **[DevX] React Client Generator**: Major enhancements to generated React client code
  - Replaced placeholder PKCE implementation with full SHA256-based solution
  - Implemented proper response validation (previously placeholder)
  - Enhanced README with comprehensive feature documentation
  - Improved error messages and validation details
  - Better security documentation and best practices

- **[DevX] Operation ID Sanitization**: Improved identifier generation
  - Enhanced `sanitize_identifier` function to handle complex operation IDs
  - Better handling of parentheses, slashes, hyphens in operation IDs
  - Proper camelCase conversion with word boundary detection

### Fixed

- **[DevX] TypeScript Empty Object Types**: Fixed formatting issue where empty object schemas generated invalid TypeScript
  - Empty objects now correctly generate as `[key: string]: any;` instead of malformed `Record<string, any>}`

- **[DevX] DELETE Operations with Query Params**: Fixed missing query parameter support in DELETE operations

- **[DevX] Duplicate Operation IDs**: Fixed duplicate operation ID handling by appending numeric suffixes

- **[DevX] PKCE Code Challenge**: Fixed PKCE implementation to use proper SHA256 hash instead of plain encoding

- **[Contracts][DevX] Response Validation**: Replaced placeholder with actual implementation (type checking, structure validation)

### Security

- **[DevX] Added comprehensive security warnings** for OAuth2 client secrets in browser code
- **[DevX] Added XSS vulnerability warnings** for localStorage token storage
- **[DevX] Implemented CSRF protection** via state parameter validation
- **[DevX] Added token expiration checking** to prevent use of expired tokens
- **[DevX] Documented security best practices** in generated client README

## [0.2.4] - 2025-01-27

### Fixed

- **[DevX] Fix request body parameter generation** in React/Vue/Svelte client generators - request bodies now correctly generate `data` parameter and `body: JSON.stringify(data)` in API client methods
- **[DevX] Fix required vs optional field handling** in generated TypeScript interfaces - required fields no longer incorrectly marked with optional marker (`?`)
- **[DevX] Fix OpenAPI serde deserialization** by adding `#[serde(rename)]` attributes for `operationId` and `requestBody` fields
- **[DevX] Apply required fields processing consistently** across all client generators (React, Vue, Svelte)

### Added

- **[DevX] Comprehensive test coverage** for request body parameter scenarios (POST, PUT, PATCH, DELETE)
- **[DevX] Test cases for `$ref` schemas** in request bodies
- **[DevX] Test cases for YAML spec support** verification

## [0.2.3] - 2025-01-27

### Fixed

- **[DevX] Fix OpenAPI example extraction** to prioritize explicit examples from schema and properties
- **[DevX] Fix request body parameter generation** in React client generator for POST, PUT, PATCH, DELETE methods
- **[DevX] Fix Handlebars template logic** for request body type generation in client code
- **[DevX] Fix useCallback dependency array formatting** in React hooks template
- **[DevX] Add comprehensive test coverage** for request body parameter scenarios

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
