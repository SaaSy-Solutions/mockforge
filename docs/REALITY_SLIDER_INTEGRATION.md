# Reality Slider Integration Guide

## Overview

The Reality Slider provides unified control over chaos engineering, latency injection, and MockAI subsystems. This document explains how the RealityEngine integrates with these subsystems and how to apply configurations.

## Architecture

The `RealityEngine` acts as a **configuration provider** rather than directly controlling subsystems. It provides configurations that should be applied to:

1. **Chaos Engineering** - Error injection, delays, timeouts
2. **Latency Injection** - Network delay simulation
3. **MockAI** - Intelligent mock behavior

## Integration Points

### 1. Server Initialization

When starting the server, the RealityEngine configuration is automatically applied to `ServerConfig`:

```rust
// In build_server_config_from_cli
if let Some(level_value) = serve_args.reality_level {
    if let Some(level) = RealityLevel::from_value(level_value) {
        config.reality.level = level;
        config.reality.enabled = true;

        // Apply reality configuration to subsystems
        let reality_engine = RealityEngine::with_level(level);
        reality_engine.apply_to_config(&mut config).await;
    }
}
```

### 2. Runtime Level Changes

When the reality level is changed via the API (`PUT /__mockforge/reality/level`), the `RealityEngine` updates its internal configuration. However, **subsystems are initialized at server startup** and may require a restart or hot-reload to apply changes.

**Current Behavior:**
- The `RealityEngine` configuration is updated immediately
- Subsystems continue using their startup configuration
- For full effect, restart the server or implement hot-reload

**Future Enhancement:**
To support runtime updates without restart, subsystems would need to:
1. Store references in `AdminState`
2. Provide update methods that accept new configurations
3. Be called from the `set_reality_level` handler

### 3. Workspace Persistence

The `reality_level` is automatically persisted with workspaces:

```rust
// In WorkspaceConfig
pub struct WorkspaceConfig {
    // ... other fields ...
    pub reality_level: Option<RealityLevel>,
}
```

When a workspace is saved/loaded, the `reality_level` is included automatically since `Workspace` is serialized as a whole.

## Applying Reality Configurations

### Method: `apply_to_config`

The `RealityEngine::apply_to_config()` method updates a `ServerConfig` with settings from the current reality level:

```rust
let reality_engine = RealityEngine::with_level(RealityLevel::ModerateRealism);
reality_engine.apply_to_config(&mut server_config).await;
```

This method:
- Updates chaos engineering configuration (if enabled)
- Updates latency profile in `config.core.default_latency`
- Updates MockAI configuration in `config.mockai`

### Configuration Mapping

| Reality Level | Chaos Error Rate | Latency (ms) | MockAI |
|--------------|-----------------|--------------|--------|
| 1: Static Stubs | 0% | 0 | Disabled |
| 2: Light Simulation | 0% | 10-50 | Enabled |
| 3: Moderate Realism | 5% | 50-200 | Enabled |
| 4: High Realism | 10% | 100-500 | Enabled |
| 5: Production Chaos | 15% | 200-2000 | Enabled |

## Hot-Reload Support (Future)

To support runtime reality level changes without restart:

1. **Store subsystem references in AdminState:**
   ```rust
   pub struct AdminState {
       pub reality_engine: Arc<RwLock<RealityEngine>>,
       pub chaos_engine: Option<Arc<RwLock<ChaosEngine>>>,  // NEW
       pub latency_injector: Option<Arc<RwLock<LatencyInjector>>>,  // NEW
       pub mockai: Option<Arc<RwLock<MockAI>>>,  // NEW
   }
   ```

2. **Add update methods to subsystems:**
   ```rust
   impl ChaosEngine {
       pub async fn update_config(&self, config: ChaosConfig) {
           *self.config.write().await = config;
       }
   }
   ```

3. **Update set_reality_level handler:**
   ```rust
   pub async fn set_reality_level(...) {
       let config = engine.get_config().await;

       // Update subsystems
       if let Some(chaos) = &state.chaos_engine {
           chaos.update_config(config.chaos).await;
       }
       // ... update other subsystems
   }
   ```

## Best Practices

1. **Startup Configuration**: Always apply reality configuration during server initialization
2. **Workspace Loading**: When loading a workspace, check `workspace.config.reality_level` and apply it
3. **API Changes**: Document that runtime level changes may require server restart for full effect
4. **Testing**: Test each reality level to ensure subsystems behave as expected

## Example: Loading Workspace with Reality Level

```rust
// Load workspace
let workspace = persistence.load_workspace(&workspace_id).await?;

// Apply workspace reality level if set
if let Some(level) = workspace.config.reality_level {
    let reality_engine = RealityEngine::with_level(level);
    reality_engine.apply_to_config(&mut server_config).await;
}
```

## Troubleshooting

**Issue**: Reality level changes don't affect behavior
- **Solution**: Restart the server or implement hot-reload support

**Issue**: Workspace reality level not persisting
- **Solution**: Ensure `WorkspaceConfig` includes `reality_level` field (already implemented)

**Issue**: Subsystems not using reality configuration
- **Solution**: Call `apply_to_config()` during server initialization
