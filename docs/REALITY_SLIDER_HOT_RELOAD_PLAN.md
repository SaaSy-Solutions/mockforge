# Reality Slider Hot-Reload Implementation Plan

## Overview

This document outlines the plan to implement hot-reload support for the Reality Slider, allowing runtime changes to chaos, latency, and MockAI subsystems without requiring a server restart.

## Current Architecture

### Subsystems
1. **Chaos Engine** (`ChaosEngine`)
   - ✅ Has `update_config()` method
   - ✅ Config stored in `Arc<RwLock<ChaosConfig>>`
   - ❌ Middleware stored in static `OnceLock` (not updatable)

2. **Latency Injector** (`LatencyInjector`)
   - ✅ Has `set_enabled()` method
   - ❌ No `update_profile()` method
   - ❌ Profile stored directly, not in `Arc<RwLock>>`

3. **MockAI** (`MockAI`)
   - ✅ Has `config` field
   - ❌ No `update_config()` method
   - ❌ Config not in `Arc<RwLock>>`

4. **AdminState**
   - ✅ Has `reality_engine: Arc<RwLock<RealityEngine>>`
   - ❌ No references to chaos/latency/mockai subsystems

## Implementation Plan

### Phase 1: Add Update Methods to Subsystems

#### 1.1 LatencyInjector - Add Profile Update Method

**File**: `crates/mockforge-core/src/latency.rs`

```rust
impl LatencyInjector {
    /// Update latency profile at runtime
    pub fn update_profile(&mut self, profile: LatencyProfile) {
        self.latency_profile = profile;
    }

    /// Update latency profile (async version for Arc<RwLock>)
    pub async fn update_profile_async(
        self: &Arc<RwLock<Self>>,
        profile: LatencyProfile,
    ) {
        let mut injector = self.write().await;
        injector.update_profile(profile);
    }
}
```

#### 1.2 MockAI - Add Config Update Method

**File**: `crates/mockforge-core/src/intelligent_behavior/mockai.rs`

```rust
impl MockAI {
    /// Update configuration at runtime
    pub fn update_config(&mut self, config: IntelligentBehaviorConfig) {
        self.config = config;
        // Update components that depend on config
        let behavior_config = self.config.behavior_model.clone();
        self.validation_generator = ValidationGenerator::new(behavior_config.clone());
        self.pagination_intelligence = PaginationIntelligence::new(behavior_config);
    }

    /// Update configuration (async version for Arc<RwLock>)
    pub async fn update_config_async(
        self: &Arc<RwLock<Self>>,
        config: IntelligentBehaviorConfig,
    ) {
        let mut mockai = self.write().await;
        mockai.update_config(config);
    }
}
```

### Phase 2: Store Subsystem References in AdminState

#### 2.1 Update AdminState Structure

**File**: `crates/mockforge-ui/src/handlers.rs`

```rust
pub struct AdminState {
    // ... existing fields ...

    /// Reality engine for managing realism levels
    pub reality_engine: Arc<RwLock<mockforge_core::RealityEngine>>,

    // NEW: Subsystem references for hot-reload
    /// Chaos API state (contains config that can be updated)
    pub chaos_api_state: Option<Arc<mockforge_chaos::api::ChaosApiState>>,

    /// Latency injector for HTTP middleware
    pub latency_injector: Option<Arc<RwLock<mockforge_core::latency::LatencyInjector>>>,

    /// MockAI instance
    pub mockai: Option<Arc<RwLock<mockforge_core::intelligent_behavior::MockAI>>>,
}
```

#### 2.2 Update AdminState::new()

```rust
impl AdminState {
    pub fn new(
        // ... existing params ...
        chaos_api_state: Option<Arc<mockforge_chaos::api::ChaosApiState>>,
        latency_injector: Option<Arc<RwLock<mockforge_core::latency::LatencyInjector>>>,
        mockai: Option<Arc<RwLock<mockforge_core::intelligent_behavior::MockAI>>>,
    ) -> Self {
        Self {
            // ... existing fields ...
            chaos_api_state,
            latency_injector,
            mockai,
        }
    }
}
```

### Phase 3: Pass Subsystem References from CLI

#### 3.1 Update handle_serve() Function

**File**: `crates/mockforge-cli/src/main.rs`

```rust
async fn handle_serve(serve_args: ServeArgs) -> Result<()> {
    // ... existing code ...

    // Create chaos API router (already done)
    let (chaos_router, chaos_config_arc, latency_tracker) =
        create_chaos_api_router(chaos_config.clone(), mockai.clone());

    // Extract ChaosApiState from router state (need to add getter)
    // OR: Store references before creating router

    // Create latency injector if needed
    let latency_injector = if config.core.latency_enabled {
        Some(Arc::new(RwLock::new(
            mockforge_core::latency::LatencyInjector::new(
                config.core.default_latency.clone(),
                Default::default(),
            )
        )))
    } else {
        None
    };

    // Pass to admin server
    let admin_state = AdminState::new(
        // ... existing params ...
        Some(chaos_api_state),  // Need to extract from router
        latency_injector,
        mockai.clone(),
    );
}
```

**Challenge**: `ChaosApiState` is created inside `create_chaos_api_router()` and not returned. We need to either:
- Return `ChaosApiState` from `create_chaos_api_router()`
- Or create it separately and pass to router creation

### Phase 4: Update set_reality_level Handler

#### 4.1 Enhanced Handler Implementation

**File**: `crates/mockforge-ui/src/handlers.rs`

```rust
pub async fn set_reality_level(
    State(state): State<AdminState>,
    Json(request): Json<SetRealityLevelRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let level = match mockforge_core::RealityLevel::from_value(request.level) {
        Some(l) => l,
        None => {
            return Json(ApiResponse::error(
                format!("Invalid reality level: {}. Must be between 1 and 5.", request.level),
            ));
        }
    };

    // Update reality engine
    {
        let mut engine = state.reality_engine.write().await;
        engine.set_level(level).await;
    }

    // Get new configuration
    let reality_config = {
        let engine = state.reality_engine.read().await;
        engine.get_config().await
    };

    // Apply to subsystems
    let mut errors = Vec::new();

    // 1. Update Chaos Engine
    if let Some(chaos_state) = &state.chaos_api_state {
        let mut chaos_config = chaos_state.config.write().await;

        // Convert RealityConfig::ChaosConfig to ChaosConfig
        *chaos_config = convert_reality_chaos_to_chaos_config(&reality_config.chaos);

        // Update middleware components (if accessible)
        // Note: ChaosMiddleware uses static OnceLock, may need refactoring
    } else {
        errors.push("Chaos engine not available".to_string());
    }

    // 2. Update Latency Injector
    if let Some(latency_injector) = &state.latency_injector {
        let mut injector = latency_injector.write().await;
        injector.update_profile(reality_config.latency.clone());
        injector.set_enabled(reality_config.latency.base_ms > 0);
    } else {
        errors.push("Latency injector not available".to_string());
    }

    // 3. Update MockAI
    if let Some(mockai) = &state.mockai {
        let mut mockai_instance = mockai.write().await;
        mockai_instance.update_config(reality_config.mockai.clone());
    } else {
        errors.push("MockAI not available".to_string());
    }

    // Return response
    if errors.is_empty() {
        Json(ApiResponse::success(serde_json::json!({
            "level": level.value(),
            "level_name": level.name(),
            "description": level.description(),
            "applied": true,
            "chaos": {
                "enabled": reality_config.chaos.enabled,
                "error_rate": reality_config.chaos.error_rate,
            },
            "latency": {
                "base_ms": reality_config.latency.base_ms,
            },
            "mockai": {
                "enabled": reality_config.mockai.enabled,
            },
        })))
    } else {
        Json(ApiResponse::error(format!(
            "Level updated but some subsystems failed: {}",
            errors.join(", ")
        )))
    }
}
```

### Phase 5: Handle ChaosMiddleware Static Issue

#### 5.1 Option A: Refactor Middleware to Use Shared Config

**File**: `crates/mockforge-chaos/src/middleware.rs`

Instead of storing config in middleware, read from shared state:

```rust
pub struct ChaosMiddleware {
    // Remove: config stored here
    // Add: reference to shared config
    config: Arc<RwLock<ChaosConfig>>,
    // ... other fields ...
}

impl ChaosMiddleware {
    pub fn new(
        config: Arc<RwLock<ChaosConfig>>,  // Changed from ChaosConfig
        latency_tracker: Arc<LatencyMetricsTracker>,
    ) -> Self {
        // Read config when needed, don't store copy
    }

    pub async fn process_request(&self, ...) {
        let config = self.config.read().await;  // Always read latest
        // ... use config ...
    }
}
```

#### 5.2 Option B: Store Middleware in AdminState

Store `ChaosMiddleware` in `AdminState` instead of static, update when config changes:

```rust
pub struct AdminState {
    // ...
    pub chaos_middleware: Option<Arc<ChaosMiddleware>>,
}

// Update middleware when config changes
if let Some(middleware) = &state.chaos_middleware {
    // Recreate middleware with new config
    // OR: Update middleware's internal state
}
```

**Recommendation**: Option A is cleaner - middleware should read from shared config.

### Phase 6: Helper Functions

#### 6.1 Convert Reality Configs to Subsystem Configs

**File**: `crates/mockforge-core/src/reality.rs`

```rust
impl RealityEngine {
    /// Convert RealityConfig::ChaosConfig to mockforge_chaos::ChaosConfig
    pub fn convert_chaos_config(
        reality_chaos: &ChaosConfig,
    ) -> mockforge_chaos::ChaosConfig {
        // Map fields appropriately
    }
}
```

## Implementation Steps

1. ✅ **Phase 1.1**: Add `update_profile()` to `LatencyInjector`
2. ✅ **Phase 1.2**: Add `update_config()` to `MockAI`
3. ✅ **Phase 2**: Update `AdminState` to store subsystem references
4. ✅ **Phase 3**: Pass references from CLI to `AdminState`
5. ✅ **Phase 4**: Update `set_reality_level` handler
6. ✅ **Phase 5**: Refactor `ChaosMiddleware` to use shared config
7. ✅ **Phase 6**: Add conversion helpers

## Testing Strategy

1. **Unit Tests**: Test update methods on each subsystem
2. **Integration Tests**: Test `set_reality_level` API endpoint
3. **E2E Tests**:
   - Change level via API
   - Verify chaos behavior changes
   - Verify latency changes
   - Verify MockAI enabled/disabled

## Rollout Plan

1. **Week 1**: Implement Phases 1-2 (update methods, AdminState)
2. **Week 2**: Implement Phases 3-4 (CLI integration, handler update)
3. **Week 3**: Implement Phase 5 (middleware refactoring)
4. **Week 4**: Testing and bug fixes

## Risks and Mitigations

1. **Risk**: ChaosMiddleware static prevents updates
   - **Mitigation**: Refactor to use shared config (Phase 5)

2. **Risk**: Race conditions when updating multiple subsystems
   - **Mitigation**: Use proper locking, update atomically where possible

3. **Risk**: Performance impact of reading config on every request
   - **Mitigation**: Config is in `Arc<RwLock>`, reads are cheap

4. **Risk**: Breaking changes to existing API
   - **Mitigation**: Maintain backward compatibility, add new fields as optional

## Success Criteria

- ✅ Reality level can be changed via API without restart
- ✅ Chaos behavior updates immediately
- ✅ Latency profile updates immediately
- ✅ MockAI enabled/disabled immediately
- ✅ No performance degradation
- ✅ All existing tests pass
