# MockForge Comprehensive Deep Dive Findings and Implementation Plan

**Generated:** 2025-12-27
**Scope:** Full codebase analysis across 42 crates, UI frontend, and registry server

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Critical Findings](#critical-findings)
3. [Registry Server Issues](#registry-server-issues)
4. [Protocol Implementation Gaps](#protocol-implementation-gaps)
5. [UI/UX Issues](#uiux-issues)
6. [Core Crate Issues](#core-crate-issues)
7. [Security Concerns](#security-concerns)
8. [Integration Gaps](#integration-gaps)
9. [Implementation Plan](#implementation-plan)
10. [Priority Matrix](#priority-matrix)

---

## Executive Summary

MockForge is a comprehensive, multi-protocol API mocking framework with **42 crates** organized into logical layers. The codebase demonstrates substantial implementation maturity with extensive features across HTTP, gRPC, WebSocket, GraphQL, Kafka, MQTT, AMQP, SMTP, FTP, and TCP protocols.

### Overall Production Readiness: **~80%**

**Key Strengths:**
- Well-organized workspace with clear dependency layers
- Comprehensive protocol support
- Robust plugin system with WASM sandboxing
- AI-powered features (data generation, response synthesis)
- Extensive observability and metrics

**Critical Gaps:**
- Security vulnerabilities in registry server
- Incomplete protocol features (AMQP delivery, MQTT session restore)
- UI accessibility and mobile responsiveness issues
- Multiple unfinished integrations

---

## Critical Findings

### P0 - Immediate Attention Required

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| 1 | SMTP Email Provider Not Implemented | `registry-server/src/email.rs:96-101` | Emails silently not sent |
| 2 | AMQP Consumer Delivery Not Started | `mockforge-amqp/src/connection.rs:1082` | Messages not pushed to consumers |
| 3 | 2FA Secret Storage Missing | `registry-server/handlers/two_factor.rs:112-131` | 2FA setup flow broken |
| 4 | MQTT Retained Messages on Session Restore | `mockforge-mqtt/src/server.rs:249` | Restored sessions miss retained messages |

### P1 - High Priority

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| 5 | No TLS Support in AMQP/MQTT | Both crates | All connections plaintext |
| 6 | No Authentication Validation | AMQP/MQTT brokers | Any credentials accepted |
| 7 | Trial Org Analytics Hardcoded | `registry-server/handlers/analytics.rs:420` | Analytics dashboard incomplete |
| 8 | Template Expansion Migration Incomplete | `mockforge-core/src/template_expansion.rs:34` | Functions throw unimplemented!() |
| 9 | HTTP Bridge Not Complete | `mockforge-grpc/src/dynamic/http_bridge/mod.rs:377-461` | gRPC-HTTP bridging broken |
| 10 | UI Mobile Responsiveness | Multiple frontend files | Poor mobile experience |

---

## Registry Server Issues

### Authentication & Authorization

#### 2FA Implementation Gap
**File:** `crates/mockforge-registry-server/src/handlers/two_factor.rs:112-131`

```rust
// Current: Returns error, no temporary secret storage
pub async fn verify_2fa_setup(
    ...
) -> impl IntoResponse {
    // In production, you'd retrieve the secret from a temporary store (Redis/session)
    // For now, we'll require the secret to be passed in the request (less secure workaround exists)
    return (StatusCode::BAD_REQUEST, Json(json!({"error": "Use verify_2fa_setup_with_secret endpoint"})));
}
```

**Fix Required:**
- Implement Redis/session-based temporary secret storage during 2FA setup
- Store secret on `generate_2fa_secret`, retrieve on `verify_2fa_setup`

#### JWT Secret Handling
**File:** `crates/mockforge-registry-server/src/middleware/mod.rs:78`

```rust
let secret = std::env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
```

**Issue:** Reads from environment on every request instead of from AppState.

### Email System

#### SMTP Provider Not Implemented
**File:** `crates/mockforge-registry-server/src/email.rs:96-101`

```rust
EmailProvider::Smtp => {
    tracing::warn!("SMTP email provider not yet implemented, email not sent");
    Ok(())
}
```

**Impact:** If SMTP is configured, all emails are silently dropped.

### Database

#### Single Migration File
**File:** `crates/mockforge-registry-server/migrations/20250101000001_init.sql`

- Contains all tables in one 500+ line file
- Should be split for proper version control and rollback capability

### Hardcoded Values

| Value | Location | Current | Should Be |
|-------|----------|---------|-----------|
| Email from | `email.rs:79` | `noreply@mockforge.dev` | Configurable |
| Support email | `handlers/support.rs:172-173` | `support@mockforge.dev` | Configurable |
| App URLs | Multiple email templates | `https://app.mockforge.dev` | Configurable |
| TOTP time step | `two_factor.rs` | 30 seconds | Configurable |
| TOTP digits | `two_factor.rs` | 6 digits | Configurable |
| Analytics DB path | `main.rs:106` | `mockforge-analytics.db` | Already configurable (just defaulted) |

---

## Protocol Implementation Gaps

### AMQP (mockforge-amqp)

#### Missing: Consumer Message Delivery
**File:** `crates/mockforge-amqp/src/connection.rs:1082`

```rust
// TODO: Start delivering messages to this consumer
// This would typically spawn a task to deliver messages
```

**Impact:** Consumers are registered but messages are never pushed to them. Only polling via `Basic.Get` works.

#### Missing Features
- `Basic.Deliver` - push messages to consumers
- Exchange-to-exchange bindings (stubs only)
- Dead letter exchange support
- Priority queues
- Alternate exchanges
- Queue/message TTL enforcement
- Connection.Blocked/Unblocked

#### Security Issues
- No authentication validation (accepts any credentials)
- No TLS support
- No authorization/ACL
- No rate limiting

### MQTT (mockforge-mqtt)

#### Missing: Retained Messages on Session Restore
**File:** `crates/mockforge-mqtt/src/server.rs:249`

```rust
// TODO: Deliver retained messages for restored subscriptions
```

**Impact:** Clients with persistent sessions don't receive retained messages for their existing subscriptions after reconnecting.

#### Missing Features
- MQTT 5.0 support (enum exists but not implemented)
- Will message delivery on disconnect
- Shared subscriptions
- Session persistence to disk
- User property support

#### Security Issues
- Same as AMQP: no auth validation, no TLS, no ACL, no rate limiting

### gRPC (mockforge-grpc)

#### HTTP Bridge Incomplete
**File:** `crates/mockforge-grpc/src/dynamic/http_bridge/mod.rs:377-461`

Multiple TODOs and dead_code markers for:
- Handler factory implementation
- Bridge stats endpoint

#### Schema/Reflection Features
**Files:** `smart_mock_generator.rs:150-151`, `schema_graph.rs:151-157`

- Range-based field inference not implemented
- Entity extraction features not implemented

---

## UI/UX Issues

### Mobile Responsiveness

| Component | Issue | File |
|-----------|-------|------|
| Sidebar | Fixed width (w-64), not collapsible | `Layout.tsx` |
| Monaco editors | Fixed heights | Multiple files |
| Navigation | No mobile menu | `Layout.tsx` |
| Dialogs | Not optimized for mobile | `ConfirmDialog.tsx` |

### Accessibility (WCAG Compliance)

| Issue | Impact | Priority |
|-------|--------|----------|
| Missing focus traps in dialogs | Focus escapes modals | High |
| No Escape key handler for dialogs | Keyboard users stuck | High |
| No skip-to-content link | Screen reader navigation | Medium |
| Color contrast in some areas | Low vision users | Medium |
| Monaco editor accessibility | Screen reader support unclear | Medium |
| Missing aria-live for toasts | Announcements not made | Low |

### Missing Loading States

| Page | Loading State Present |
|------|----------------------|
| Dashboard | Yes |
| ConfigEditor | Yes |
| EndpointBuilder | **No** (when fetching endpoint) |
| ApiDocs | **No** |

### Protocol Support Gaps in UI

#### Dashboard
- `by_protocol` stats only show http, grpc, websocket, graphql - **missing mqtt, smtp**
- `getProtocolIcon` doesn't handle graphql, mqtt, smtp (returns default Globe)
- `getProtocolColor` doesn't handle graphql, mqtt, smtp (returns gray)

#### Endpoint Forms Feature Parity

| Feature | HTTP | gRPC | WebSocket | GraphQL | MQTT | SMTP |
|---------|------|------|-----------|---------|------|------|
| Static response | Yes | Yes | Yes | Yes | Yes | Yes |
| Template response | Yes | No | No | Yes | Yes | Yes |
| Faker response | Yes | No | No | Yes | Yes | Yes |
| AI response | Yes | No | No | No | No | No |
| Latency config | Yes | No | No | Yes | Yes | Yes |
| Failure injection | Yes | No | No | Yes | No | No |
| Behavior config | Yes | No | No | Yes | Yes | Yes |

### Incomplete Features

| Feature | Status | Location |
|---------|--------|----------|
| YAML parsing in import | Placeholder only | `Dashboard.tsx` |
| Format conversion | Changes highlighting only, no content conversion | `ConfigEditor.tsx` |
| Zustand store | Defined but never used | `useEndpointStore.ts` |
| Dark mode toggle | Styles ready, no toggle | All |
| Search/filter | Not implemented | Dashboard |
| Pagination | Not implemented | Dashboard |
| Bulk operations | Not implemented | Dashboard |
| Endpoint testing | Not implemented | All |

### ErrorBoundary Integration
- Only at app level - component errors crash entire view
- Should add component-level boundaries for forms

---

## Core Crate Issues

### mockforge-core

#### unimplemented!() Calls
**File:** `src/template_expansion.rs:34`
```rust
pub fn expand_templates_in_json(_value: &serde_json::Value) -> serde_json::Value {
    unimplemented!("This function has been moved to the mockforge-template-expansion crate")
}
```

**File:** `src/ai_response.rs:187`
```rust
unimplemented!("expand_prompt_template has been moved to mockforge-template-expansion crate")
```

**Impact:** Calling these functions causes panics.

#### AI Contract Diff TODOs
**File:** `src/ai_contract_diff/diff_analyzer.rs:393,416,462`
- Reference resolution for OpenAPI schemas not implemented

### mockforge-cli

#### VBR Commands
**File:** `src/vbr_commands.rs:440`
```rust
// TODO: Initialize session manager
```

#### Snapshot Commands
**File:** `src/snapshot_commands.rs:117,159`
- Integration with server state not complete

#### Plugin Commands
**File:** `src/plugin_commands.rs:705-1220`
- Multiple TODO placeholders for plugin logic

### mockforge-collab

#### Cloud Storage APIs
**File:** `src/backup.rs:676,712`
```rust
// TODO: Update to newer versions of azure_storage_blobs and google-cloud-storage APIs
```

### mockforge-bench

#### Security Payloads
**File:** `src/security_payloads.rs:367-368`
- LDAP injection payloads not implemented
- XXE payloads not implemented

**File:** `src/command.rs:1068`
- k6 output parsing not implemented

### mockforge-scenarios

**File:** `src/reality_profile_pack.rs:532,548`
- Persona registry not integrated
- Chaos config deserialization incomplete

---

## Security Concerns

### Critical

| Issue | Location | Risk |
|-------|----------|------|
| No TLS in AMQP/MQTT | Both crates | Data exposed in transit |
| No auth validation in AMQP/MQTT | Both crates | Any client can connect |
| SMTP provider silent failure | registry-server/email.rs | Emails lost without notice |

### High

| Issue | Location | Risk |
|-------|----------|------|
| Excessive unwrap/expect calls | ~7,554 total | Potential panics |
| JWT secret from env on each request | middleware/mod.rs | Inconsistent, could fail |
| No rate limiting in protocol mocks | AMQP/MQTT | Resource exhaustion |

### Medium

| Issue | Location | Risk |
|-------|----------|------|
| Hardcoded domain names | Email templates | Inflexible deployment |
| FFI unsafe blocks | mockforge-sdk/ffi.rs | Memory safety (well-documented) |
| Master key file permissions | mockforge-core/encryption.rs | Key exposure on Linux |

### panic!/unwrap/expect Statistics

| Type | Count | Notes |
|------|-------|-------|
| `.unwrap()` | ~7,554 | Many in tests, some in production |
| `.expect()` | ~598 | Many in tests, some in production |
| `panic!()` | ~100+ | Mostly in tests and error cases |
| `unimplemented!()` | 3 | In migration stubs |
| `unreachable!()` | ~15 | In exhaustive matches |

---

## Integration Gaps

### Plugin System
- Not integrated with most protocols (only CLI has full integration)
- Protocol crates don't expose plugin hooks

### Observability
**Missing from:**
- mockforge-kafka
- mockforge-mqtt (now has metrics.rs - NEW)
- mockforge-amqp (now has metrics.rs - NEW)
- mockforge-ftp
- mockforge-tcp
- mockforge-smtp

### World State
- Only integrated with HTTP
- Other protocols not connected

### Collaboration Features
- Not integrated with CLI
- Isolated from main workflow

---

## Implementation Plan

### Phase 1: Critical Fixes (P0)

#### 1.1 Implement SMTP Email Provider
**Effort:** 2-3 hours
**File:** `crates/mockforge-registry-server/src/email.rs`

```rust
EmailProvider::Smtp => {
    let creds = Credentials::new(
        self.smtp_username.clone().unwrap_or_default(),
        self.smtp_password.clone().unwrap_or_default(),
    );

    let mailer = SmtpTransport::relay(&self.smtp_host.clone().unwrap_or_default())?
        .credentials(creds)
        .build();

    mailer.send(&email)?;
    Ok(())
}
```

#### 1.2 Implement AMQP Consumer Delivery
**Effort:** 4-6 hours
**File:** `crates/mockforge-amqp/src/connection.rs`

Tasks:
1. Create consumer delivery task spawner
2. Implement message routing from queue to consumer
3. Handle consumer cancellation
4. Add delivery acknowledgment tracking

#### 1.3 Implement 2FA Secret Temporary Storage
**Effort:** 2-3 hours
**Files:** `crates/mockforge-registry-server/src/handlers/two_factor.rs`, `auth.rs`

Tasks:
1. Add Redis-based temporary storage for TOTP secrets
2. Store secret with 5-minute expiration on generate
3. Retrieve and verify on setup completion
4. Delete after successful verification

#### 1.4 Implement MQTT Retained Message Delivery on Session Restore
**Effort:** 2-3 hours
**File:** `crates/mockforge-mqtt/src/server.rs`

Tasks:
1. On session restore, get client's existing subscriptions
2. For each subscription, check for matching retained messages
3. Deliver retained messages to restored client

### Phase 2: High Priority (P1)

#### 2.1 Add TLS Support to AMQP/MQTT
**Effort:** 1-2 days each
**Files:** Both broker implementations

Tasks:
1. Add rustls dependency
2. Implement TLS acceptor wrapper
3. Add TLS configuration options
4. Update tests

#### 2.2 Complete gRPC HTTP Bridge
**Effort:** 1-2 days
**File:** `crates/mockforge-grpc/src/dynamic/http_bridge/mod.rs`

Tasks:
1. Implement handler factory
2. Implement stats endpoint
3. Add integration tests

#### 2.3 Complete Template Expansion Migration
**Effort:** 2-4 hours
**File:** `crates/mockforge-core/src/template_expansion.rs`

Tasks:
1. Either remove the stub functions
2. Or forward to mockforge-template-expansion crate

#### 2.4 Fix UI Mobile Responsiveness
**Effort:** 1 day
**Files:** `Layout.tsx`, all form components

Tasks:
1. Add collapsible sidebar
2. Add mobile hamburger menu
3. Make Monaco editors responsive
4. Optimize dialogs for mobile

#### 2.5 Add UI Protocol Support Parity
**Effort:** 4-6 hours
**Files:** `Dashboard.tsx`, endpoint forms

Tasks:
1. Add mqtt, smtp to protocol stats
2. Add icons and colors for all protocols
3. Add Template/Faker/AI to gRPC/WebSocket forms

### Phase 3: Medium Priority (P2)

#### 3.1 UI Accessibility Improvements
**Effort:** 1 day

Tasks:
1. Add focus traps to all dialogs
2. Add Escape key handlers
3. Add skip-to-content link
4. Review and fix color contrast
5. Add aria-live regions for toasts

#### 3.2 Add Authentication to AMQP/MQTT
**Effort:** 1 day each

Tasks:
1. Add credential validation callbacks
2. Implement simple username/password checking
3. Add configurable auth requirements

#### 3.3 Complete Plugin Commands in CLI
**Effort:** 2-3 days
**File:** `crates/mockforge-cli/src/plugin_commands.rs`

Tasks:
1. Implement remaining TODO items
2. Add proper error handling
3. Add integration tests

#### 3.4 Update Cloud Storage APIs
**Effort:** 4-6 hours
**File:** `crates/mockforge-collab/src/backup.rs`

Tasks:
1. Update azure_storage_blobs API calls
2. Update google-cloud-storage API calls
3. Test with actual cloud providers

#### 3.5 Split Registry Server Migrations
**Effort:** 2-3 hours

Tasks:
1. Split init.sql into logical migrations
2. Add proper version numbering
3. Test migration order

### Phase 4: Lower Priority (P3)

#### 4.1 Add Missing Loading States to UI
**Effort:** 2-3 hours

#### 4.2 Implement Dark Mode Toggle
**Effort:** 2-3 hours

#### 4.3 Add Search/Filter/Pagination to Dashboard
**Effort:** 4-6 hours

#### 4.4 Complete Security Payloads in Bench
**Effort:** 2-3 hours

#### 4.5 Remove/Fix Unused Zustand Store
**Effort:** 30 minutes

#### 4.6 Add Component-Level Error Boundaries
**Effort:** 2-3 hours

---

## Priority Matrix

### P0 - Critical (Do First)

| Task | Effort | Impact |
|------|--------|--------|
| 1.1 SMTP Email Provider | 2-3h | Emails actually sent |
| 1.2 AMQP Consumer Delivery | 4-6h | Core AMQP functionality |
| 1.3 2FA Secret Storage | 2-3h | Auth flow works |
| 1.4 MQTT Retained Messages | 2-3h | Core MQTT functionality |

### P1 - High (This Sprint)

| Task | Effort | Impact |
|------|--------|--------|
| 2.1 TLS Support | 2-4 days | Security baseline |
| 2.2 gRPC HTTP Bridge | 1-2 days | Feature complete |
| 2.3 Template Migration | 2-4h | No more panics |
| 2.4 Mobile Responsiveness | 1 day | Mobile users |
| 2.5 UI Protocol Parity | 4-6h | Consistent UX |

### P2 - Medium (Next Sprint)

| Task | Effort | Impact |
|------|--------|--------|
| 3.1 Accessibility | 1 day | WCAG compliance |
| 3.2 AMQP/MQTT Auth | 2 days | Security |
| 3.3 Plugin Commands | 2-3 days | Feature complete |
| 3.4 Cloud Storage APIs | 4-6h | Cloud backup works |
| 3.5 Split Migrations | 2-3h | Better DB mgmt |

### P3 - Lower (Backlog)

| Task | Effort | Impact |
|------|--------|--------|
| 4.1 Loading States | 2-3h | Better UX |
| 4.2 Dark Mode Toggle | 2-3h | User preference |
| 4.3 Dashboard Features | 4-6h | Better UX |
| 4.4 Security Payloads | 2-3h | Complete testing |
| 4.5 Remove Unused Store | 30m | Clean code |
| 4.6 Error Boundaries | 2-3h | Better error handling |

---

## Appendix: File Reference

### Registry Server Key Files
- `crates/mockforge-registry-server/src/main.rs`
- `crates/mockforge-registry-server/src/handlers/two_factor.rs`
- `crates/mockforge-registry-server/src/email.rs`
- `crates/mockforge-registry-server/src/middleware/mod.rs`
- `crates/mockforge-registry-server/src/middleware/rate_limit.rs`

### AMQP Key Files
- `crates/mockforge-amqp/src/connection.rs` (NEW - 2005 lines)
- `crates/mockforge-amqp/src/metrics.rs` (NEW - 351 lines)
- `crates/mockforge-amqp/src/broker.rs`
- `crates/mockforge-amqp/src/protocol.rs`

### MQTT Key Files
- `crates/mockforge-mqtt/src/server.rs`
- `crates/mockforge-mqtt/src/session.rs` (NEW - 992 lines)
- `crates/mockforge-mqtt/src/protocol.rs` (NEW - 1249 lines)
- `crates/mockforge-mqtt/src/metrics.rs` (NEW - 322 lines)

### UI Key Files
- `ui-builder/frontend/src/App.tsx`
- `ui-builder/frontend/src/pages/Dashboard.tsx`
- `ui-builder/frontend/src/pages/EndpointBuilder.tsx`
- `ui-builder/frontend/src/components/Layout.tsx`
- `ui-builder/frontend/src/lib/api.ts`

### Core Key Files
- `crates/mockforge-core/src/template_expansion.rs`
- `crates/mockforge-core/src/ai_response.rs`
- `crates/mockforge-grpc/src/dynamic/http_bridge/mod.rs`
- `crates/mockforge-cli/src/plugin_commands.rs`
