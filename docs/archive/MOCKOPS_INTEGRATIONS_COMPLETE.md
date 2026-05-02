# MockOps Platform - Integration Status

**Last Updated:** 2025-01-27

## Integration Summary

Integration work for the MockOps Platform has been completed for the promotion service. Additional integrations are documented below.

## ✅ Completed Integrations

### 1. Promotion Service Event Emission

**Location:** `crates/mockforge-collab/src/promotion.rs`

**Changes:**
- Added `mockforge-pipelines` as optional dependency (feature: `pipelines`)
- Emit `promotion.completed` event when promotion status changes to `Completed`
- Event includes:
  - Promotion ID
  - Entity type (scenario/persona/config)
  - From/to environment
  - Workspace ID

**Code:**
```rust
// In update_promotion_status method
if status == PromotionStatus::Completed {
    #[cfg(feature = "pipelines")]
    {
        use mockforge_pipelines::events::{publish_event, PipelineEvent};
        // ... emit event
    }
}
```

**Usage:**
Enable the `pipelines` feature in `mockforge-collab` to activate event emission:
```toml
[dependencies]
mockforge-collab = { version = "0.3.2", features = ["pipelines"] }
```

## 📝 Integration Notes

### 2. Drift Detection Event Emission

**Status:** Architecture Decision Required

**Issue:** Circular dependency between `mockforge-core` (drift GitOps) and `mockforge-pipelines`

**Options:**
1. **Callback Pattern** - Pass event emitter callback to drift handler
2. **Event Bus Abstraction** - Create trait in core, implement in pipelines
3. **Caller-Level Emission** - Emit events from code that calls drift handler

**Recommended:** Option 3 - Emit events at the caller level (e.g., in UI handlers or service layer)

**Location for Integration:**
- `crates/mockforge-ui/src/handlers/` - API handlers that trigger drift detection
- `crates/mockforge-collab/src/` - Service layer that manages drift budgets

### 3. Schema Change Detection Event Emission

**Status:** Pending Implementation

**Location:** `crates/mockforge-recorder/src/sync.rs`

**Approach:**
- Detect schema changes during sync operations
- Emit `schema.changed` event when OpenAPI/Protobuf schemas are modified
- Include schema type and change details in event payload

**Integration Points:**
- `SyncService::sync_once()` - When schema differences detected
- `SyncGitOps::process_sync_changes()` - When schema updates in PR

### 4. Scenario Published Event Emission

**Status:** Pending Implementation

**Location:** `crates/mockforge-registry-server/src/handlers/scenarios.rs`

**Approach:**
- Emit `scenario.published` event in `publish_scenario` handler
- Include scenario ID, name, version, workspace ID

**Note:** Registry server is separate from main MockForge codebase, may need different integration approach (webhook, message queue, or direct dependency)

## 🔧 Step Integrations

### Auto-Promote Step

**Status:** Structure Complete, Integration Pending

**Location:** `crates/mockforge-pipelines/src/steps/auto_promote.rs`

**Needs:**
- Access to `PromotionService` instance
- Workspace ID and entity details from event
- Environment mapping (from/to)

**Integration Approach:**
- Pass `PromotionService` to pipeline executor during initialization
- Or use dependency injection pattern
- Or make step executor configurable with service instances

### Regenerate SDK Step

**Status:** Structure Complete, Integration Pending

**Location:** `crates/mockforge-pipelines/src/steps/regenerate_sdk.rs`

**Needs:**
- Integration with `mockforge-sdk` crate
- Language-specific SDK generation
- Workspace context

**Integration Approach:**
- Add `mockforge-sdk` as dependency to `mockforge-pipelines`
- Call SDK generation functions with workspace context
- Handle generation errors gracefully

### Notify Step

**Status:** Structure Complete, Integration Pending

**Location:** `crates/mockforge-pipelines/src/steps/notify.rs`

**Needs:**
- Slack API integration
- Email/SMTP integration
- Webhook HTTP client

**Integration Approach:**
- Add Slack SDK (e.g., `slack-morphism` or direct HTTP)
- Add SMTP client (e.g., `lettre`)
- Use existing `reqwest` for webhooks

## 🚀 Next Steps

### Immediate

1. **Complete Promotion Integration**
   - Test event emission
   - Verify pipeline triggers on promotion completion

2. **Implement Caller-Level Event Emission**
   - Add event emission in UI handlers for drift detection
   - Add event emission in sync service for schema changes

3. **Complete Step Integrations**
   - Integrate auto-promote with promotion service
   - Integrate SDK regeneration
   - Integrate notification services

### Short Term

1. **Registry Server Integration**
   - Determine integration approach (webhook vs direct)
   - Implement scenario published event emission

2. **Testing**
   - Integration tests for event emission
   - End-to-end tests for pipeline execution
   - Test step integrations

### Long Term

1. **Event Persistence**
   - Store events in database for audit trail
   - Event replay capability

2. **Event Filtering**
   - Advanced event filtering in pipeline triggers
   - Event transformation before pipeline execution

## Architecture Decisions

### Event Emission Pattern

**Decision:** Use feature flags for optional pipeline integration

**Rationale:**
- Avoids circular dependencies
- Allows MockForge to work without pipelines
- Enables gradual adoption

**Implementation:**
- `#[cfg(feature = "pipelines")]` guards around event emission
- Optional dependency on `mockforge-pipelines`
- Graceful degradation when feature not enabled

### Step Executor Pattern

**Decision:** Trait-based step executors with dependency injection

**Rationale:**
- Flexible and extensible
- Easy to test
- Clear separation of concerns

**Implementation:**
- `PipelineStepExecutor` trait
- Step executors registered with pipeline executor
- Services passed via step context or executor initialization

## Testing Strategy

### Unit Tests
- Event emission in promotion service
- Pipeline step execution
- Event matching in pipeline triggers

### Integration Tests
- End-to-end pipeline execution
- Event flow from source to pipeline
- Step integration with services

### E2E Tests
- Complete promotion → pipeline → notification flow
- Drift detection → pipeline → PR creation flow
- Schema change → pipeline → SDK regeneration flow
