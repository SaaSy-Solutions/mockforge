# Critical Fixes - Detailed Remediation Guide

This document provides specific code-level remediation steps for the 8 critical issues identified in the comprehensive improvement plan.

---

## 1. Fix Broken Mutation Testing Workflow

**File:** `.github/workflows/mutation-testing.yml`
**Line:** 34
**Severity:** CRITICAL - CI/CD Broken

### Current Code
```yaml
strategy:
  fail-fast: false
  matrix:
    crate:
      - mockforge-core
      - mockforge-http
      - mockforge-openapi  # Does not exist!
```

### Fix
Remove the non-existent crate from the matrix:
```yaml
strategy:
  fail-fast: false
  matrix:
    crate:
      - mockforge-core
      - mockforge-http
      - mockforge-schema  # Or remove entirely
```

### Verification
```bash
ls crates/ | grep mockforge-openapi  # Should return nothing
cargo check -p mockforge-schema      # Verify alternative exists
```

---

## 2. Fix GraphQL Cache Serialization

**File:** `crates/mockforge-graphql/src/cache.rs`
**Lines:** 61-88
**Severity:** CRITICAL - Feature Non-Functional

### Current Code (Broken)
```rust
impl CachedResponse {
    pub fn from_response(response: &Response) -> Self {
        // Since Response doesn't implement Serialize, we extract what we can
        Self {
            data: serde_json::Value::Null,  // Always returns Null!
            errors: None,
            extensions: None,
        }
    }

    pub fn to_response(&self) -> Response {
        // Returns empty response
        Response::new(serde_json::Value::Null)  // Always returns Null!
    }
}
```

### Fix
```rust
impl CachedResponse {
    pub fn from_response(response: &Response) -> Self {
        Self {
            data: response.data.clone(),
            errors: response.errors.as_ref().map(|errs| {
                errs.iter()
                    .map(|e| CachedError {
                        message: e.message.clone(),
                        path: e.path.clone(),
                        extensions: e.extensions.clone(),
                    })
                    .collect()
            }),
            extensions: response.extensions.clone(),
        }
    }

    pub fn to_response(&self) -> Response {
        let mut response = Response::new(self.data.clone());
        if let Some(errors) = &self.errors {
            response.errors = errors
                .iter()
                .map(|e| async_graphql::ServerError {
                    message: e.message.clone(),
                    source: None,
                    locations: vec![],
                    path: e.path.clone().unwrap_or_default(),
                    extensions: e.extensions.clone(),
                })
                .collect();
        }
        if let Some(ext) = &self.extensions {
            response.extensions = ext.clone();
        }
        response
    }
}
```

### Additional Types Needed
```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct CachedError {
    pub message: String,
    pub path: Option<Vec<async_graphql::PathSegment>>,
    pub extensions: Option<async_graphql::ErrorExtensionValues>,
}
```

---

## 3. Implement Snapshot Protocol State Capture

**File:** `crates/mockforge-core/src/snapshots/manager.rs`
**Line:** 164
**Severity:** CRITICAL - Data Loss Risk

### Current Code (Broken)
```rust
// TODO: Get protocol state from engine when protocol adapters are integrated
let protocol_path = protocols_dir.join(format!("{}.json", protocol_name));
let empty_state = serde_json::json!({});  // Always empty!
fs::write(&protocol_path, serde_json::to_string_pretty(&empty_state)?).await?;
```

### Fix
```rust
/// Protocol state provider trait for snapshot integration
pub trait ProtocolStateProvider: Send + Sync {
    fn get_state(&self) -> Result<serde_json::Value, SnapshotError>;
    fn restore_state(&self, state: serde_json::Value) -> Result<(), SnapshotError>;
}

// In SnapshotManager
pub struct SnapshotManager {
    // ... existing fields
    protocol_providers: HashMap<String, Arc<dyn ProtocolStateProvider>>,
}

impl SnapshotManager {
    pub fn register_protocol(&mut self, name: &str, provider: Arc<dyn ProtocolStateProvider>) {
        self.protocol_providers.insert(name.to_string(), provider);
    }

    async fn capture_protocol_state(&self, protocol_name: &str, protocols_dir: &Path) -> Result<(), SnapshotError> {
        let protocol_path = protocols_dir.join(format!("{}.json", protocol_name));

        let state = if let Some(provider) = self.protocol_providers.get(protocol_name) {
            provider.get_state()?
        } else {
            tracing::warn!("No state provider for protocol {}, capturing empty state", protocol_name);
            serde_json::json!({})
        };

        fs::write(&protocol_path, serde_json::to_string_pretty(&state)?).await?;
        Ok(())
    }
}
```

### Protocol Adapter Implementation Example (MQTT)
```rust
impl ProtocolStateProvider for MqttBroker {
    fn get_state(&self) -> Result<serde_json::Value, SnapshotError> {
        let state = MqttState {
            sessions: self.sessions.read().clone(),
            retained_messages: self.retained.read().clone(),
            subscriptions: self.subscriptions.read().clone(),
        };
        serde_json::to_value(state).map_err(|e| SnapshotError::Serialization(e.to_string()))
    }

    fn restore_state(&self, state: serde_json::Value) -> Result<(), SnapshotError> {
        let mqtt_state: MqttState = serde_json::from_value(state)
            .map_err(|e| SnapshotError::Deserialization(e.to_string()))?;

        *self.sessions.write() = mqtt_state.sessions;
        *self.retained.write() = mqtt_state.retained_messages;
        *self.subscriptions.write() = mqtt_state.subscriptions;
        Ok(())
    }
}
```

---

## 4. Implement OpenAPI $ref Resolution

**File:** `crates/mockforge-core/src/ai_contract_diff/diff_analyzer.rs`
**Lines:** 393, 416, 462
**Severity:** CRITICAL - Schema Comparison Broken

### Current Code (Broken)
```rust
// For references, return empty schema (TODO: resolve references)
ReferenceOr::Reference { reference } => {
    // Just return empty for now
    serde_json::json!({})
}
```

### Fix - Add Reference Resolver
```rust
use std::collections::HashMap;

pub struct SchemaResolver<'a> {
    components: Option<&'a openapiv3::Components>,
    cache: HashMap<String, serde_json::Value>,
}

impl<'a> SchemaResolver<'a> {
    pub fn new(components: Option<&'a openapiv3::Components>) -> Self {
        Self {
            components,
            cache: HashMap::new(),
        }
    }

    pub fn resolve_reference(&mut self, reference: &str) -> Result<serde_json::Value, DiffError> {
        // Check cache first
        if let Some(cached) = self.cache.get(reference) {
            return Ok(cached.clone());
        }

        // Parse reference: #/components/schemas/User
        let parts: Vec<&str> = reference.trim_start_matches("#/").split('/').collect();

        if parts.len() < 3 || parts[0] != "components" {
            return Err(DiffError::InvalidReference(reference.to_string()));
        }

        let schema = match (parts[1], self.components) {
            ("schemas", Some(components)) => {
                components.schemas.get(parts[2])
                    .ok_or_else(|| DiffError::SchemaNotFound(parts[2].to_string()))?
            }
            _ => return Err(DiffError::UnsupportedReference(reference.to_string())),
        };

        let resolved = self.resolve_schema_or_ref(schema)?;
        self.cache.insert(reference.to_string(), resolved.clone());
        Ok(resolved)
    }

    pub fn resolve_schema_or_ref(&mut self, schema: &ReferenceOr<Schema>) -> Result<serde_json::Value, DiffError> {
        match schema {
            ReferenceOr::Reference { reference } => self.resolve_reference(reference),
            ReferenceOr::Item(schema) => self.schema_to_json(schema),
        }
    }

    fn schema_to_json(&mut self, schema: &Schema) -> Result<serde_json::Value, DiffError> {
        // Recursively resolve any nested references
        let mut json = serde_json::to_value(schema)
            .map_err(|e| DiffError::Serialization(e.to_string()))?;

        self.resolve_nested_refs(&mut json)?;
        Ok(json)
    }

    fn resolve_nested_refs(&mut self, value: &mut serde_json::Value) -> Result<(), DiffError> {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::String(ref_str)) = map.get("$ref") {
                    let resolved = self.resolve_reference(ref_str)?;
                    *value = resolved;
                } else {
                    for (_, v) in map.iter_mut() {
                        self.resolve_nested_refs(v)?;
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.resolve_nested_refs(item)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

### Update DiffAnalyzer Usage
```rust
impl DiffAnalyzer {
    pub fn analyze(&self, old_spec: &OpenAPI, new_spec: &OpenAPI) -> DiffResult {
        let mut old_resolver = SchemaResolver::new(old_spec.components.as_ref());
        let mut new_resolver = SchemaResolver::new(new_spec.components.as_ref());

        // Now use resolvers when comparing schemas
        for (path, old_item) in &old_spec.paths.paths {
            // ...
            let old_schema = old_resolver.resolve_schema_or_ref(&old_response.schema)?;
            let new_schema = new_resolver.resolve_schema_or_ref(&new_response.schema)?;
            // Compare resolved schemas
        }
    }
}
```

---

## 5. Store Security Risk Reviewer

**File:** `crates/mockforge-core/src/security/risk_assessment.rs`
**Line:** 623
**Severity:** CRITICAL - Compliance Gap

### Current Code (Broken)
```rust
let _ = reviewed_by; // TODO: Store reviewer
```

### Fix
```rust
// Add to RiskAssessment struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskReview {
    pub assessment_id: Uuid,
    pub reviewed_by: String,
    pub reviewed_at: DateTime<Utc>,
    pub decision: ReviewDecision,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewDecision {
    Approved,
    Rejected,
    NeedsMoreInfo,
}

// Update the review method
impl RiskAssessmentEngine {
    pub async fn record_review(
        &self,
        assessment_id: Uuid,
        reviewed_by: &str,
        decision: ReviewDecision,
        notes: Option<String>,
    ) -> Result<RiskReview, RiskError> {
        let review = RiskReview {
            assessment_id,
            reviewed_by: reviewed_by.to_string(),
            reviewed_at: Utc::now(),
            decision,
            notes,
        };

        // Store in database/persistence layer
        self.review_store.save(&review).await?;

        // Emit audit event
        self.audit_emitter.emit(AuditEvent::RiskReviewed {
            assessment_id,
            reviewer: reviewed_by.to_string(),
            decision: decision.clone(),
            timestamp: review.reviewed_at,
        }).await?;

        Ok(review)
    }
}
```

---

## 6. Fix K8s Operator Cron Parsing

**File:** `crates/mockforge-k8s-operator/src/webhook.rs`
**Lines:** 169-173
**Severity:** CRITICAL - Scheduling Broken

### Current Code (Broken)
```rust
fn is_valid_cron(expr: &str) -> bool {
    // In production, use a proper cron parser
    // For now, just check it's not empty
    !expr.is_empty()  // Always true for any non-empty string!
}
```

### Fix - Add Proper Cron Parsing
Add to `Cargo.toml`:
```toml
[dependencies]
cron = "0.12"
```

```rust
use cron::Schedule;
use std::str::FromStr;

fn is_valid_cron(expr: &str) -> bool {
    if expr.is_empty() {
        return false;
    }

    // Try to parse as standard cron (5 fields) or extended (6 fields with seconds)
    Schedule::from_str(expr).is_ok()
}

fn parse_cron_schedule(expr: &str) -> Result<Schedule, CronError> {
    Schedule::from_str(expr).map_err(|e| CronError::InvalidExpression {
        expression: expr.to_string(),
        error: e.to_string(),
    })
}

fn next_execution_time(expr: &str) -> Result<DateTime<Utc>, CronError> {
    let schedule = parse_cron_schedule(expr)?;
    schedule
        .upcoming(Utc)
        .next()
        .ok_or(CronError::NoUpcomingExecution)
}
```

### Update Reconciler
**File:** `crates/mockforge-k8s-operator/src/reconciler.rs:221-226`

```rust
// Replace simplified implementation
async fn check_scheduled_execution(&self, mock: &MockForgeInstance) -> Result<bool, Error> {
    if let Some(schedule) = &mock.spec.schedule {
        let cron_schedule = Schedule::from_str(schedule)
            .map_err(|e| Error::InvalidSchedule(e.to_string()))?;

        let last_run = mock.status.as_ref()
            .and_then(|s| s.last_scheduled_run)
            .unwrap_or(DateTime::<Utc>::MIN_UTC);

        // Check if there's a scheduled time between last run and now
        for next_time in cron_schedule.after(&last_run) {
            if next_time <= Utc::now() {
                return Ok(true);
            }
            if next_time > Utc::now() {
                break;
            }
        }
    }
    Ok(false)
}
```

---

## 7. Implement Tunnel Providers (or Document as Planned)

**File:** `crates/mockforge-tunnel/src/manager.rs`
**Lines:** 34-47
**Severity:** CRITICAL - Advertised Feature Non-Functional

### Option A: Implement ngrok Provider
```rust
use ngrok::prelude::*;

pub struct NgrokProvider {
    authtoken: Option<String>,
}

impl NgrokProvider {
    pub fn new(authtoken: Option<String>) -> Self {
        Self { authtoken }
    }
}

#[async_trait]
impl TunnelProvider for NgrokProvider {
    async fn create_tunnel(&self, config: &TunnelConfig) -> Result<TunnelInfo, TunnelError> {
        let mut builder = ngrok::Session::builder();

        if let Some(token) = &self.authtoken {
            builder = builder.authtoken(token);
        }

        let session = builder.connect().await
            .map_err(|e| TunnelError::ConnectionFailed(e.to_string()))?;

        let tunnel = session
            .http_endpoint()
            .listen()
            .await
            .map_err(|e| TunnelError::TunnelCreationFailed(e.to_string()))?;

        Ok(TunnelInfo {
            id: tunnel.id().to_string(),
            public_url: tunnel.url().to_string(),
            local_addr: config.local_addr.clone(),
            provider: "ngrok".to_string(),
        })
    }

    async fn close_tunnel(&self, tunnel_id: &str) -> Result<(), TunnelError> {
        // ngrok tunnels close when the session drops
        // For persistent tunnels, track sessions in a map
        Ok(())
    }
}
```

### Option B: Mark as Planned Feature
If not implementing, update the error to be clear:
```rust
TunnelProvider::Ngrok => {
    Err(TunnelError::ProviderNotAvailable {
        provider: "ngrok".to_string(),
        reason: "ngrok integration is planned for v1.1. Use 'self-hosted' provider instead.".to_string(),
        tracking_issue: Some("https://github.com/mockforge/mockforge/issues/XXX".to_string()),
    })
}
```

And update documentation to clearly list supported providers.

---

## 8. Fix UUID Fallback in Collab

**File:** `crates/mockforge-collab/src/core_bridge.rs`
**Line:** 73
**Severity:** CRITICAL - Data Corruption Risk

### Current Code (Dangerous)
```rust
team_workspace.id = Uuid::parse_str(&core_workspace.id)
    .unwrap_or_else(|_| Uuid::new_v4()); // Silently creates new ID!
```

### Fix - Proper Error Handling
```rust
impl CoreBridge {
    pub fn convert_workspace(&self, core_workspace: &CoreWorkspace) -> Result<TeamWorkspace, BridgeError> {
        let id = Uuid::parse_str(&core_workspace.id)
            .map_err(|e| BridgeError::InvalidWorkspaceId {
                workspace_id: core_workspace.id.clone(),
                parse_error: e.to_string(),
            })?;

        Ok(TeamWorkspace {
            id,
            name: core_workspace.name.clone(),
            // ... other fields
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Invalid workspace ID '{workspace_id}': {parse_error}")]
    InvalidWorkspaceId {
        workspace_id: String,
        parse_error: String,
    },
    // ... other errors
}
```

### Alternative - Validate at Creation
If the core workspace ID should always be a valid UUID, add validation at the source:
```rust
impl CoreWorkspace {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),  // Ensure valid UUID at creation
            name: name.to_string(),
            // ...
        }
    }

    pub fn id_as_uuid(&self) -> Result<Uuid, uuid::Error> {
        Uuid::parse_str(&self.id)
    }
}
```

---

## Verification Checklist

After applying all fixes, run these verification steps:

```bash
# 1. Verify workflow fix
act -j mutation-testing --dryrun

# 2. Run GraphQL cache tests
cargo test -p mockforge-graphql cache

# 3. Test snapshot with protocol state
cargo test -p mockforge-core snapshot_protocol_state

# 4. Test OpenAPI reference resolution
cargo test -p mockforge-core ref_resolution

# 5. Verify cron parsing
cargo test -p mockforge-k8s-operator cron

# 6. Test tunnel providers
cargo test -p mockforge-tunnel providers

# 7. Test collab UUID handling
cargo test -p mockforge-collab uuid_validation

# 8. Full integration test
cargo test --workspace
```

---

## Timeline Estimate

| Fix | Complexity | Estimated Hours |
|-----|------------|-----------------|
| 1. Workflow fix | Simple | 0.5 |
| 2. GraphQL cache | Medium | 4 |
| 3. Snapshot protocol state | Complex | 8 |
| 4. OpenAPI $ref resolution | Complex | 8 |
| 5. Store reviewer | Simple | 2 |
| 6. K8s cron parsing | Medium | 4 |
| 7. Tunnel providers | Medium-Complex | 8-16 |
| 8. UUID fallback | Simple | 2 |
| **Total** | | **36-44 hours** |

---

*Document Version: 1.0*
*Last Updated: 2025-12-27*
