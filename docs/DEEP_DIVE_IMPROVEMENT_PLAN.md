# MockForge Comprehensive Improvement Plan

**Generated:** 2025-12-27
**Version:** 0.3.16

This document synthesizes findings from a deep dive analysis of the MockForge codebase, identifying incomplete implementations, integration gaps, production readiness issues, UI/UX problems, and suggestions for improvement.

---

## Executive Summary

MockForge is a feature-rich API mocking framework with **42 crates** covering multiple protocols (HTTP, gRPC, GraphQL, MQTT, AMQP, Kafka, SMTP). While the codebase demonstrates strong architectural foundations, this analysis identified:

- **~119 TODO/FIXME comments** requiring attention
- **18 orphaned registry server handlers** not wired to routes
- **Critical protocol gaps** (gRPC streaming, MQTT 5.0)
- **Security vulnerabilities** requiring immediate attention
- **35 registry handlers with minimal test coverage**
- **Multiple UI/UX issues** in the frontend

---

## Table of Contents

1. [Critical Issues (P0)](#1-critical-issues-p0)
2. [High Priority Issues (P1)](#2-high-priority-issues-p1)
3. [Medium Priority Issues (P2)](#3-medium-priority-issues-p2)
4. [Low Priority Issues (P3)](#4-low-priority-issues-p3)
5. [Suggested Improvements](#5-suggested-improvements)
6. [Implementation Roadmap](#6-implementation-roadmap)

---

## 1. Critical Issues (P0)

### 1.1 Security Vulnerabilities

| Issue | Location | Description | Fix |
|-------|----------|-------------|-----|
| **SQL Injection Risk** | `crates/mockforge-vbr/src/handlers.rs:128,308,435` | Field names from user JSON used directly in SQL column names | Validate field names against strict allowlist before SQL construction |
| **Environment Default Risk** | `crates/mockforge-registry-server/src/middleware/security_headers.rs` | ENVIRONMENT defaults to "development", enabling permissive CSP | Require explicit ENVIRONMENT=production or fail-safe to strict CSP |
| **CSRF Bypass** | `crates/mockforge-registry-server/src/middleware/csrf.rs:165-173` | Requests without Origin/Referer headers are allowed through | Consider requiring Origin header on state-changing requests |
| **JWT Validation** | `crates/mockforge-registry-server/src/auth.rs:118-122` | Missing audience/issuer validation in JWT | Add `aud` and `iss` validation to prevent token misuse |
| **Panic in Production** | `crates/mockforge-registry-server/src/main.rs:199` | `expect()` call on Ctrl+C handler setup | Replace with proper error handling |

### 1.2 Incomplete Protocol Implementations Blocking Real Usage

| Protocol | Missing Feature | Impact | Location |
|----------|----------------|--------|----------|
| **gRPC** | Server/Client/Bidirectional Streaming | Cannot mock streaming gRPC services | `crates/mockforge-grpc/src/dynamic/mod.rs` |
| **gRPC** | HTTP Bridge Streaming | REST-to-gRPC scenarios fail | `crates/mockforge-grpc/src/dynamic/http_bridge/mod.rs:377-461` |
| **AMQP** | Transaction Support (Tx.Select/Commit/Rollback) | Cannot test transactional messaging | `crates/mockforge-amqp/src/protocol.rs` |
| **HTTP** | XPath Matching | Cannot mock XML/SOAP APIs properly | `crates/mockforge-http/src/management.rs` |

### 1.3 Missing Test Coverage for Security-Critical Code

| Component | Files | Issue |
|-----------|-------|-------|
| **Auth Handlers** | `handlers/auth.rs`, `handlers/oauth.rs`, `handlers/sso.rs`, `handlers/two_factor.rs` | Zero handler-level tests for authentication flows |
| **Billing** | `handlers/billing.rs` | No tests for payment processing |
| **GDPR** | `handlers/gdpr.rs` | No tests for data export/deletion |

---

## 2. High Priority Issues (P1)

### 2.1 Orphaned Registry Server Handlers

**18 handler modules exist but are NOT exposed in `mod.rs` or wired to routes:**

| Handler | Purpose | Action Required |
|---------|---------|-----------------|
| `audit.rs` | Audit logging | Wire to routes |
| `billing.rs` | Stripe subscriptions | Wire to routes |
| `faq.rs` | FAQ management | Wire to routes |
| `gdpr.rs` | GDPR compliance | Wire to routes |
| `hosted_mocks.rs` | Fly.io deployment | Wire to routes |
| `legal.rs` | Legal documents | Wire to routes |
| `scenarios.rs` | Scenario management | Wire to routes |
| `scenario_reviews.rs` | Scenario reviews | Wire to routes |
| `security.rs` | Security features | Wire to routes |
| `settings.rs` | Settings management | Wire to routes |
| `status.rs` | Status endpoints | Wire to routes |
| `templates.rs` | Template management | Wire to routes |
| `template_reviews.rs` | Template reviews | Wire to routes |
| `tokens.rs` | Token management | Wire to routes |
| `token_rotation.rs` | Token rotation | Wire to routes |
| `usage.rs` | Usage tracking | Wire to routes |
| `verification.rs` | Verification | Wire to routes |
| `support.rs` | Support tickets | Wire to routes |

### 2.2 Cloud Storage Integration Broken

| Service | Location | Issue |
|---------|----------|-------|
| **Azure Blob Storage** | `crates/mockforge-collab/src/backup.rs:676-684` | Needs API update for azure_storage_blobs 0.19 |
| **Google Cloud Storage** | `crates/mockforge-collab/src/backup.rs:712-729` | Needs API update for google-cloud-storage 1.4.0 |

### 2.3 Production Readiness Gaps

| Gap | Location | Fix |
|-----|----------|-----|
| **No per-user rate limiting** | `middleware/rate_limit.rs` | Rate limiting is per-IP only; add per-user limits |
| **Missing readiness probe DB check** | Health endpoints | Ensure `/health/ready` checks database connectivity |
| **No circuit breaker** | External service calls | Add circuit breaker pattern for Redis, S3, etc. |
| **Inconsistent timeouts** | Various HTTP clients | Standardize timeout configuration across all clients |

### 2.4 CLI Incomplete Features

| Feature | Location | Status |
|---------|----------|--------|
| Plugin search | `cli/src/plugin_commands.rs:326` | Returns "not yet implemented" |
| OAuth login | `cli/src/cloud_commands.rs:375` | Returns "not yet implemented" |
| Interactive login | `cli/src/cloud_commands.rs:421` | Returns "not yet implemented" |
| Backend generators (non-Rust) | `cli/src/backend_generator.rs:99` | Only rust-axum implemented |
| FTP fixtures commands | `cli/src/ftp_commands.rs:194-202` | Not implemented |
| Voice STT | `cli/src/speech_to_text.rs:655` | Placeholder only |

---

## 3. Medium Priority Issues (P2)

### 3.1 UI/UX Issues

| Issue | Location | Fix |
|-------|----------|-----|
| **No error states for data fetching** | `Dashboard.tsx`, `ConfigEditor.tsx`, `ApiDocs.tsx` | Add `isError` handling from React Query |
| **No loading fallback for Monaco Editor** | All endpoint forms | Add loading skeleton |
| **Missing inline form validation** | `GrpcEndpointForm.tsx`, `WebsocketEndpointForm.tsx`, etc. | Add Zod validation with inline errors |
| **Poor accessibility** | Tab components, collapsible sections | Add proper ARIA attributes and keyboard navigation |
| **No pagination** | `Dashboard.tsx` endpoints list | Add pagination for large lists |
| **Search not debounced** | `Dashboard.tsx` | Add debounce to search input |
| **Hard-coded strings** | All components | Implement i18n with react-i18next |
| **Type safety issues** | Various (`any` types, type casting) | Replace with proper TypeScript interfaces |

### 3.2 Protocol Implementation Gaps

| Protocol | Feature | Status |
|----------|---------|--------|
| **MQTT** | MQTT 5.0 features (session expiry, topic aliases, shared subscriptions) | Not implemented |
| **MQTT** | $SYS topics | Not implemented |
| **AMQP** | Flow Control (Channel.Flow) | Not implemented |
| **AMQP** | Queue.Purge | Not implemented |
| **HTTP** | Request/Response Inspection | Returns NOT_IMPLEMENTED |
| **HTTP** | Snapshot Diff | Returns NOT_IMPLEMENTED |
| **HTTP** | Drift Budget | Returns NOT_IMPLEMENTED |
| **GraphQL** | Apollo Federation | Needs verification |

### 3.3 Documentation Gaps

| Gap | Location | Action |
|-----|----------|--------|
| **Missing env vars doc** | Referenced but doesn't exist | Create `docs/ENVIRONMENT_VARIABLES.md` |
| **Undocumented config options** | `config.rs` has options not in template | Add to `config.template.yaml` |
| **Minimal rustdoc** | `mockforge-federation`, `mockforge-performance`, `mockforge-pipelines` | Add documentation |
| **Deprecated items without migration** | `template_expansion.rs`, `ai_response.rs`, `domains.rs` | Add migration guides or remove |

### 3.4 Architectural Issues

| Issue | Location | Fix |
|-------|----------|-----|
| **mockforge-chaos too large** | 52+ modules | Split into focused crates |
| **mockforge-core too large** | 60+ public modules | Extract domain-specific modules |
| **Duplicate functionality** | Template expansion in two crates | Consolidate |
| **36 `#[allow(dead_code)]` annotations** | Various | Clean up or implement |

---

## 4. Low Priority Issues (P3)

### 4.1 Placeholder Implementations

| Component | Location | Issue |
|-----------|----------|-------|
| RAG keyword search | `data/src/rag/engine.rs:475` | Returns empty Vec |
| RAG Anthropic response | `data/src/rag/engine.rs:695` | Returns placeholder string |
| RAG Ollama response | `data/src/rag/engine.rs:757` | Returns placeholder string |
| Storage factory | `data/src/rag/storage.rs:599-614` | File/DB/Vector backends not implemented |
| Plugin security scanner | `plugin-registry/src/security.rs:210-259` | Placeholders for ClamAV/RustSec integration |
| World state aggregators | `world-state/src/aggregators/*.rs` | Multiple placeholder implementations |
| LDAP injection payloads | `bench/src/security_payloads.rs:367` | Empty Vec |
| XXE payloads | `bench/src/security_payloads.rs:368` | Empty Vec |

### 4.2 Test Infrastructure Issues

| Issue | Description |
|-------|-------------|
| ~50+ ignored tests | Require running server/infrastructure |
| Sleep-based timing | Potential flakiness in error_scenarios_tests.rs |
| No database mocks | Tests skip if no DB |
| No mock email service | Email functionality untested |
| No mock Redis | Rate limiting tests limited |
| Hardcoded ports in some tests | Risk of port conflicts |

### 4.3 Minor Security Findings

| Issue | Location | Risk |
|-------|----------|------|
| Email logged in password reset | `handlers/auth.rs:365,449` | Privacy concern (GDPR) |
| "null" origin allowed in CORS | `main.rs:248` | Potential exploitation |
| IP spoofing via X-Forwarded-For | `middleware/trusted_proxy.rs` | Requires proper proxy config |

---

## 5. Suggested Improvements

### 5.1 Feature Enhancements

| Feature | Description | Value |
|---------|-------------|-------|
| **Federation CLI commands** | Add CLI for `mockforge-federation` | Multi-workspace orchestration |
| **Reporting CLI commands** | Add CLI for `mockforge-reporting` | PDF/email reports |
| **Analytics in CLI** | Expose `mockforge-analytics` via CLI | Metrics commands |
| **Desktop app protocols** | Add GraphQL, MQTT, Kafka to desktop | Feature parity |
| **Per-user rate limiting** | Limit by user ID, not just IP | Better abuse prevention |
| **Circuit breaker** | Add for external dependencies | Resilience |

### 5.2 Developer Experience

| Improvement | Description |
|-------------|-------------|
| **CI for ignored tests** | Set up infrastructure to run integration tests |
| **Database mocks** | Create mock database layer for testing |
| **Plugin development guide** | Document plugin creation workflow |
| **Architecture diagrams** | Visual crate dependency documentation |
| **Demo video** | As noted in DOCUMENTATION_IMPROVEMENTS.md |

### 5.3 Code Quality

| Improvement | Description |
|-------------|-------------|
| **Consolidate admin checks** | Extract repeated is_owner/is_admin pattern |
| **Remove dead code** | Address 36 `#[allow(dead_code)]` annotations |
| **Fix circular dependencies** | Re-enable drift_learning functionality |
| **Type safety in frontend** | Replace `any` types with proper interfaces |

---

## 6. Implementation Roadmap

### Phase 1: Security & Stability (Critical)

1. **Fix SQL injection in VBR handlers** - Validate field names against schema
2. **Fix environment default** - Require explicit ENVIRONMENT or default to strict
3. **Add JWT audience/issuer validation**
4. **Replace expect() calls in production code**
5. **Add handler tests for auth flows**

### Phase 2: Wire Orphaned Features

1. **Export all 18 orphaned handlers in mod.rs**
2. **Wire handlers to routes in routes.rs**
3. **Add basic API documentation for new endpoints**

### Phase 3: Protocol Completion

1. **Implement gRPC streaming** (server, client, bidirectional)
2. **Implement AMQP transactions**
3. **Add MQTT 5.0 core features**
4. **Complete HTTP inspection endpoints**

### Phase 4: Production Hardening

1. **Add per-user rate limiting**
2. **Implement circuit breaker for external services**
3. **Add database connectivity to readiness probe**
4. **Standardize timeouts across HTTP clients**
5. **Update Azure/GCS storage integrations**

### Phase 5: UI/UX Polish

1. **Add error states to all data fetching**
2. **Add loading states for Monaco Editor**
3. **Implement inline form validation**
4. **Add pagination to endpoints list**
5. **Debounce search input**
6. **Fix accessibility issues**

### Phase 6: Documentation & Testing

1. **Create ENVIRONMENT_VARIABLES.md**
2. **Document missing config options**
3. **Add rustdoc to underdocumented crates**
4. **Set up CI for integration tests**
5. **Create database mocks for testing**

### Phase 7: Architecture Improvements

1. **Split mockforge-chaos into smaller crates**
2. **Consolidate template expansion**
3. **Clean up dead code**
4. **Extract modules from mockforge-core**

---

## Appendix: Issue Counts by Category

| Category | Count |
|----------|-------|
| TODO comments | 119 |
| FIXME comments | ~10 |
| Orphaned handlers | 18 |
| Security issues | 8 |
| Protocol gaps (critical) | 4 |
| UI/UX issues | 14 |
| Test gaps | 10 |
| Documentation gaps | 6 |
| Dead code annotations | 36 |
| Placeholder implementations | 15 |
| Ignored tests | 50+ |

---

## Files to Prioritize

1. `crates/mockforge-vbr/src/handlers.rs` - SQL injection fix
2. `crates/mockforge-registry-server/src/handlers/mod.rs` - Export orphaned handlers
3. `crates/mockforge-registry-server/src/routes.rs` - Wire new routes
4. `crates/mockforge-grpc/src/dynamic/mod.rs` - gRPC streaming
5. `crates/mockforge-collab/src/backup.rs` - Cloud storage APIs
6. `ui-builder/frontend/src/pages/Dashboard.tsx` - Error states, pagination
7. `docs/ENVIRONMENT_VARIABLES.md` - Create new file
