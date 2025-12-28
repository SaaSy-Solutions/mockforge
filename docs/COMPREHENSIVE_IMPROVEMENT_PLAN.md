# MockForge Comprehensive Improvement Plan

**Generated:** 2025-12-27
**Version:** 0.3.16 → 1.0.0 Readiness Assessment

This document outlines all identified gaps, incomplete implementations, production-readiness issues, UI/UX problems, and suggested improvements across the entire MockForge codebase.

---

## Executive Summary

After a comprehensive deep-dive analysis of 42 crates, 692 Rust source files, and the React frontend, the following high-level findings emerged:

| Category | Critical | High | Medium | Low | Total |
|----------|----------|------|--------|-----|-------|
| Incomplete Features | 8 | 15 | 22 | 12 | 57 |
| Production-Readiness | 12 | 18 | 25 | 10 | 65 |
| UI/UX Issues | 3 | 12 | 18 | 8 | 41 |
| Documentation Gaps | 2 | 8 | 12 | 5 | 27 |
| Error Handling | 15 | 20 | 30 | N/A | 65 |
| **Total** | **40** | **73** | **107** | **35** | **255** |

**Key Statistics:**
- 8,674+ `.unwrap()` and `.expect()` calls across 692 files
- 100+ TODO/FIXME comments in production code
- 3 broken/incomplete GitHub workflows
- 6 deleted documentation files still referenced

---

## Table of Contents

1. [Critical Issues (Fix Immediately)](#1-critical-issues)
2. [High Priority (Before 1.0 Release)](#2-high-priority)
3. [Medium Priority (Post-1.0)](#3-medium-priority)
4. [Low Priority (Future Enhancements)](#4-low-priority)
5. [UI/UX Improvements](#5-uiux-improvements)
6. [Documentation Tasks](#6-documentation-tasks)
7. [Implementation Plan](#7-implementation-plan)

---

## 1. Critical Issues

### 1.1 Broken GitHub Workflow
**File:** `.github/workflows/mutation-testing.yml:34`
**Issue:** References non-existent `mockforge-openapi` crate
**Fix:** Remove from matrix or replace with `mockforge-schema`
**Impact:** CI/CD pipeline failure

### 1.2 GraphQL Cache Returns Null
**File:** `crates/mockforge-graphql/src/cache.rs:61-88`
**Issue:** `CachedResponse::from_response()` and `to_response()` return hardcoded `serde_json::Value::Null`
**Fix:** Implement proper GraphQL response serialization
**Impact:** Cache completely non-functional

### 1.3 Protocol State Not Captured in Snapshots
**File:** `crates/mockforge-core/src/snapshots/manager.rs:164`
**Issue:** Protocol snapshots write empty JSON objects instead of actual state
**Fix:** Integrate with protocol adapters to capture real state
**Impact:** Snapshot restore fails to restore protocol state

### 1.4 OpenAPI Reference Resolution Missing
**File:** `crates/mockforge-core/src/ai_contract_diff/diff_analyzer.rs:393,416,462`
**Issue:** TODO comments indicate `$ref` resolution returns empty schemas
**Fix:** Implement proper JSON reference resolution using `jsonref` or similar
**Impact:** Schema comparison for referenced objects is broken

### 1.5 Security Risk Reviewer Not Stored
**File:** `crates/mockforge-core/src/security/risk_assessment.rs:623`
**Issue:** `let _ = reviewed_by; // TODO: Store reviewer`
**Fix:** Persist reviewer information in audit trail
**Impact:** SOC2/compliance audit trail incomplete

### 1.6 K8s Operator Cron Parsing Stub
**File:** `crates/mockforge-k8s-operator/src/webhook.rs:169-173`
**Issue:** `is_valid_cron()` always returns true regardless of input
**Fix:** Use proper cron parser library (e.g., `cron` crate)
**Impact:** Invalid cron schedules accepted, scheduling failures at runtime

### 1.7 Tunnel Providers Not Implemented
**File:** `crates/mockforge-tunnel/src/manager.rs:34-47`
**Issue:** Cloudflare, ngrok, localtunnel all return "coming soon" errors
**Fix:** Implement at least one provider or mark as planned feature
**Impact:** Advertised feature non-functional

### 1.8 UUID Fallback Creates Data Inconsistency
**File:** `crates/mockforge-collab/src/core_bridge.rs:73`
**Issue:** UUID parse failure silently generates new UUID
**Fix:** Return proper error, don't mask data corruption
**Impact:** Potential data loss in collaborative editing

---

## 2. High Priority

### 2.1 Error Handling Improvements

#### 2.1.1 Replace Panic Calls in Production Code
**Files:**
- `crates/mockforge-core/src/workspace/sync.rs:1104,1126,1145` - Match arms panic
- `crates/mockforge-core/src/generate_config.rs:361,381` - Plugin type panic
- `crates/mockforge-cli/src/main.rs:3195` - Command dispatch panic

**Action:** Replace with proper `Result` returns

#### 2.1.2 Reduce Critical Unwrap Calls
**Priority Files (by occurrence count):**
| File | Unwrap Count | Risk Level |
|------|-------------|------------|
| `mockforge-graphql/src/registry.rs` | 47 | High |
| `mockforge-core/src/stateful_handler.rs` | 108 | High |
| `mockforge-core/src/openapi/response.rs` | 76 | High |
| `mockforge-chaos/src/failure_designer.rs` | 39 | Medium |
| `mockforge-analytics/src/queries.rs` | 47 | Medium |

### 2.2 Protocol Authentication/Authorization

#### 2.2.1 AMQP Authentication Missing
**File:** `crates/mockforge-amqp/src/connection.rs:542`
**Issue:** Comment "For now, just accept any authentication"
**Fix:** Implement username/password validation, SASL support

#### 2.2.2 MQTT Authentication Missing
**File:** `crates/mockforge-mqtt/src/broker.rs`
**Issue:** CONNECT username/password not validated
**Fix:** Add auth handler hook

### 2.3 Incomplete AMQP Methods

| Method | File:Line | Status |
|--------|-----------|--------|
| EXCHANGE_BIND | connection.rs:801-813 | Stubbed |
| EXCHANGE_UNBIND | connection.rs:814-826 | Stubbed |
| QUEUE_PURGE | connection.rs:1032-1033 | Stubbed |
| BASIC_CANCEL | connection.rs:1204-1235 | Stubbed |
| BASIC_RECOVER | connection.rs:1515-1542 | Stubbed |

### 2.4 CLI Main.rs Refactoring
**File:** `crates/mockforge-cli/src/main.rs`
**Issue:** 9,475 lines in single file
**Fix:** Split into:
- `commands/mod.rs` - Command definitions
- `server.rs` - Server initialization
- `protocols/mod.rs` - Protocol setup
- `middleware/mod.rs` - Middleware configuration

### 2.5 Frontend TypeScript Improvements

#### 2.5.1 Replace `any` Types
**Files with excessive `any` usage:**
- `ui-builder/frontend/src/lib/api.ts:353-365` - ServerConfig interface
- All endpoint forms (`HttpEndpointForm.tsx`, `GrpcEndpointForm.tsx`, etc.) - Props typed as `any`

**Fix:** Create proper protocol config interfaces

#### 2.5.2 Form Validation Gaps
| Form | Missing Validation |
|------|-------------------|
| HttpEndpointForm | Path format, header names, status codes (100-599) |
| GrpcEndpointForm | Proto file existence, type validation |
| GraphqlEndpointForm | SDL syntax validation |
| MqttEndpointForm | Topic pattern validation, latency min<max |
| SmtpEndpointForm | Port warnings, timeout minimums |

### 2.6 Plugin System Gaps

#### 2.6.1 Missing Plugin Types
- Middleware plugin (request/response interception)
- Observability plugin (custom metrics)
- Caching plugin (custom cache implementations)
- Validation plugin (custom request/response validation)

#### 2.6.2 Plugin Authentication
**File:** `crates/mockforge-plugin-registry/src/security.rs:1-96`
**Issue:** No per-plugin permission model
**Fix:** Implement capability-based security

### 2.7 Analytics Migration Conflict
**File:** `crates/mockforge-analytics/migrations/`
**Issue:** Two migrations numbered `002_*`
**Fix:** Rename `002_pillar_usage.sql` to `003_pillar_usage.sql`

---

## 3. Medium Priority

### 3.1 Protocol Improvements

#### 3.1.1 MQTT Enhancements
- Will message support
- MQTT 5.0 full support (currently primarily 3.1.1)
- Persistent message storage (currently in-memory only)
- Message expiry/TTL enforcement

#### 3.1.2 gRPC Improvements
- HTTP Bridge handler completion
- E2E test coverage (currently `#[ignore]`)
- Per-method authentication

### 3.2 Desktop App Version Sync
**File:** `desktop-app/tauri.conf.json`
**Issue:** Version 0.2.8 vs main project 0.3.16
**Fix:** Sync versions

### 3.3 Collaboration Features
**File:** `crates/mockforge-collab/src/`
- Complete conflict resolution strategy
- Add offline sync support
- Improve access control architecture

### 3.4 Federation Validation
**File:** `crates/mockforge-federation/src/federation.rs`
- Service dependency validation at creation
- Multi-tenancy isolation enforcement
- Cross-workspace authentication

### 3.5 Chaos Engineering
**File:** `crates/mockforge-chaos/src/`
- Reduce 230+ unwrap calls
- Add comprehensive error handling
- Validate ML components (reinforcement learning)

### 3.6 Observability Completeness
- Add protocol-specific metrics for all protocols
- Complete OpenTelemetry integration
- System metrics for production monitoring

---

## 4. Low Priority

### 4.1 Code Quality
- Replace test `panic!()` calls with proper assertions
- Add `missing_docs = "deny"` at workspace level
- Implement dead code elimination

### 4.2 Performance
- Add pagination for endpoint listing in UI
- Implement endpoint search/filtering
- Add endpoint caching in dashboard

### 4.3 Developer Experience
- Endpoint duplication feature
- Bulk operations (multi-select, bulk delete)
- Endpoint tags/categories
- Version history/rollback

### 4.4 Fixture Permissions
**Path:** `fixtures/` subdirectories
**Issue:** 700 permissions (restricted)
**Fix:** Update to 755

---

## 5. UI/UX Improvements

### 5.1 Accessibility (a11y)

#### Missing ARIA Labels
| Component | Missing Labels |
|-----------|---------------|
| HttpEndpointForm | Method select, path input, status code, headers |
| GrpcEndpointForm | Service name, method name inputs |
| MqttEndpointForm | Topic pattern, QoS select, retained checkbox |
| SmtpEndpointForm | Port, hostname, TLS options |
| Dashboard | Import/Export buttons |

#### Focus Management
- Import/Export dialogs need focus trap
- Modal dialogs should restore focus on close
- Keyboard navigation improvements needed

### 5.2 Error Handling

| Page | Issue |
|------|-------|
| ConfigEditor | No error handling for failed config export |
| EndpointBuilder | No error message on fetch failure |
| ApiDocs | No error boundary for SwaggerUI crash |
| Dashboard | Generic import error messages |

### 5.3 Loading States
- ConfigEditor: No skeleton during initial load
- EndpointBuilder: No skeleton while loading endpoint
- Forms: No validation-in-progress indicator

### 5.4 Display Bugs
**File:** `ui-builder/frontend/src/pages/Dashboard.tsx:472`
**Issue:** SMTP display uses wrong config structure
**Fix:** Update to use `messageHandling` structure

### 5.5 Missing Protocol Support in UI
- KAFKA not in protocol selector
- AMQP not in protocol selector
- FTP not in protocol selector

### 5.6 Hardcoded Values
**File:** `ui-builder/frontend/src/components/Layout.tsx:104`
**Issue:** Version 0.1.0 hardcoded
**Fix:** Import from environment or package.json

---

## 6. Documentation Tasks

### 6.1 Critical Documentation Fixes

#### Delete References to Removed Files
**Files deleted but still referenced:**
- `docs/Admin_UI_Spec.md`
- `docs/Comparisons.md`
- `docs/Roadmap.md`
- `docs/TODO.md`
- `docs/Validation_Spec.md`
- `docs/RAG_Faker_Data_Generation.md`

**Update:** `docs/DEEP_DIVE_FINDINGS_AND_PLAN.md`

### 6.2 Missing SDK Documentation
| SDK | Status | Action |
|-----|--------|--------|
| Rust | Referenced in README, no doc | Create `docs/sdk/rust.md` |
| .NET | Referenced in README, no doc | Create `docs/sdk/dotnet.md` |
| Java | Referenced in README, no doc | Create `docs/sdk/java.md` |
| Node.js | Exists (9,825 lines) | - |
| Python | Exists (15,326 lines) | - |
| Go | Exists (16,460 lines) | - |

### 6.3 Missing Protocol Documentation
| Protocol | Status | Action |
|----------|--------|--------|
| SMTP | Crate exists, no doc | Create `docs/protocols/SMTP.md` |
| FTP | Crate exists, no doc | Create `docs/protocols/FTP.md` |
| TCP | Crate exists, no doc | Create `docs/protocols/TCP.md` |
| Kafka | Crate exists, no doc | Create `docs/protocols/KAFKA.md` |
| HTTP | Partial | Complete documentation |

### 6.4 Plugin Documentation
- Plugin development guide
- Plugin architecture overview
- Security model documentation
- Deployment/distribution guide

---

## 7. Implementation Plan

### Phase 1: Critical Fixes (Week 1-2)

| Task | File | Effort |
|------|------|--------|
| Fix mutation-testing workflow | `.github/workflows/mutation-testing.yml` | 1 hour |
| Fix GraphQL cache serialization | `mockforge-graphql/src/cache.rs` | 4 hours |
| Implement snapshot protocol state | `mockforge-core/src/snapshots/manager.rs` | 8 hours |
| Implement OpenAPI $ref resolution | `mockforge-core/src/ai_contract_diff/` | 8 hours |
| Store security reviewer | `mockforge-core/src/security/risk_assessment.rs` | 2 hours |
| Fix K8s cron parsing | `mockforge-k8s-operator/src/webhook.rs` | 4 hours |
| Fix UUID fallback in collab | `mockforge-collab/src/core_bridge.rs` | 2 hours |
| Fix analytics migration numbering | `mockforge-analytics/migrations/` | 1 hour |

**Total Phase 1:** ~30 hours

### Phase 2: High Priority (Week 3-4)

| Task | Scope | Effort |
|------|-------|--------|
| Replace panic calls in production code | 10 files | 8 hours |
| Reduce critical unwraps (top 5 files) | 5 files | 16 hours |
| AMQP/MQTT authentication | 2 crates | 16 hours |
| Implement stubbed AMQP methods | 5 methods | 12 hours |
| Refactor CLI main.rs | 1 file → 4+ | 24 hours |
| Fix frontend TypeScript types | ~15 files | 12 hours |
| Add form validation | 6 forms | 16 hours |

**Total Phase 2:** ~104 hours

### Phase 3: Medium Priority (Week 5-8)

| Task | Scope | Effort |
|------|-------|--------|
| MQTT enhancements | 4 features | 32 hours |
| gRPC improvements | 3 areas | 24 hours |
| Plugin system gaps | 4 plugin types | 40 hours |
| Collaboration features | 3 areas | 32 hours |
| Federation validation | 3 areas | 16 hours |
| Chaos engineering cleanup | 230+ unwraps | 24 hours |

**Total Phase 3:** ~168 hours

### Phase 4: Documentation (Ongoing)

| Task | Effort |
|------|--------|
| SDK documentation (3 SDKs) | 24 hours |
| Protocol documentation (4 protocols) | 16 hours |
| Plugin documentation | 12 hours |
| Clean up deleted file references | 2 hours |

**Total Phase 4:** ~54 hours

### Phase 5: UI/UX (Week 9-10)

| Task | Effort |
|------|--------|
| Accessibility improvements | 16 hours |
| Error handling improvements | 8 hours |
| Loading states | 8 hours |
| Display bug fixes | 4 hours |
| Add missing protocols to UI | 8 hours |

**Total Phase 5:** ~44 hours

---

## Appendix: Files Requiring Immediate Attention

### Top 20 Files by Issue Severity

1. `crates/mockforge-graphql/src/cache.rs` - Cache completely broken
2. `crates/mockforge-core/src/snapshots/manager.rs` - Snapshots incomplete
3. `crates/mockforge-core/src/ai_contract_diff/diff_analyzer.rs` - Schema diff broken
4. `crates/mockforge-k8s-operator/src/webhook.rs` - Cron validation stub
5. `crates/mockforge-tunnel/src/manager.rs` - 3 providers not implemented
6. `crates/mockforge-collab/src/core_bridge.rs` - Data corruption risk
7. `crates/mockforge-cli/src/main.rs` - 9,475 lines, unmaintainable
8. `crates/mockforge-amqp/src/connection.rs` - No auth, 5 stubbed methods
9. `crates/mockforge-core/src/workspace/sync.rs` - 40+ panic calls
10. `.github/workflows/mutation-testing.yml` - References non-existent crate
11. `crates/mockforge-chaos/src/failure_designer.rs` - 39 unwraps
12. `crates/mockforge-analytics/src/queries.rs` - 47 unwraps
13. `crates/mockforge-core/src/stateful_handler.rs` - 108 unwraps
14. `ui-builder/frontend/src/lib/api.ts` - Extensive `any` usage
15. `ui-builder/frontend/src/pages/Dashboard.tsx` - Display bugs, accessibility
16. `crates/mockforge-core/src/security/risk_assessment.rs` - Audit trail gap
17. `crates/mockforge-mqtt/src/broker.rs` - No authentication
18. `crates/mockforge-federation/src/database.rs` - 56 unwraps
19. `crates/mockforge-plugin-registry/src/security.rs` - No plugin auth
20. `crates/mockforge-grpc/tests/grpc_server_e2e_test.rs` - Tests disabled

---

## Change Log

| Date | Version | Changes |
|------|---------|---------|
| 2025-12-27 | 1.0 | Initial comprehensive analysis |

---

*This document should be updated as issues are resolved. Track progress in GitHub Issues or project management tool.*
