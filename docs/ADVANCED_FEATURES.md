# Advanced Features

This guide covers the advanced features in MockForge including ML-based anomaly detection, Chaos Mesh integration, multi-tenancy support, and custom chaos plugins.

## Table of Contents

- [ML-Based Anomaly Detection](#ml-based-anomaly-detection)
- [Chaos Mesh Integration](#chaos-mesh-integration)
- [Multi-Tenancy Support](#multi-tenancy-support)
- [Custom Chaos Plugins](#custom-chaos-plugins)

---

## ML-Based Anomaly Detection

MockForge includes sophisticated machine learning-based anomaly detection to identify unusual patterns in your chaos engineering experiments.

### Features

- **Statistical Outlier Detection**: Uses z-score analysis to detect values outside normal bounds
- **Trend Anomaly Detection**: Identifies sudden changes in moving averages
- **Seasonal Anomaly Detection**: Detects deviations from seasonal patterns
- **Collective Anomaly Detection**: Finds patterns across multiple metrics simultaneously

### Quick Start

```rust
use mockforge_chaos::{
    AnomalyDetector, AnomalyDetectorConfig, TimeSeriesPoint, AnomalySeverity
};
use chrono::{Utc, Duration};
use std::collections::HashMap;

// Create detector with custom configuration
let config = AnomalyDetectorConfig {
    std_dev_threshold: 3.0,
    min_baseline_samples: 30,
    moving_average_window: 10,
    enable_seasonal: true,
    seasonal_period: 24,
    sensitivity: 0.7,
};

let mut detector = AnomalyDetector::new(config);

// Add training data points
for i in 0..100 {
    let point = TimeSeriesPoint {
        timestamp: Utc::now() - Duration::hours(100 - i),
        value: 100.0 + (i as f64 * 0.5),
        metadata: HashMap::new(),
    };
    detector.add_data_point("response_time".to_string(), point);
}

// Update baseline from training data
detector.update_baseline("response_time").unwrap();

// Detect anomalies in new values
let mut context = HashMap::new();
context.insert("service".to_string(), "api-gateway".to_string());

if let Some(anomaly) = detector.detect_value_anomaly(
    "response_time",
    500.0,  // Anomalous value
    context
) {
    println!("Anomaly detected!");
    println!("Severity: {:?}", anomaly.severity);
    println!("Expected range: {:?}", anomaly.expected_range);
    println!("Deviation score: {:.2}", anomaly.deviation_score);
}

// Detect time-series anomalies
let anomalies = detector.detect_timeseries_anomalies(
    "response_time",
    24  // Look back 24 hours
).unwrap();

for anomaly in anomalies {
    println!("{:?}: {} at {}",
        anomaly.anomaly_type,
        anomaly.message,
        anomaly.timestamp
    );
}
```

### Anomaly Types

#### Statistical Outlier
Detects values that fall outside the normal statistical distribution.

```rust
// Configure sensitivity
let config = AnomalyDetectorConfig {
    std_dev_threshold: 2.5,  // Stricter threshold
    sensitivity: 0.9,        // Higher sensitivity
    ..Default::default()
};
```

#### Trend Anomaly
Identifies sudden changes in moving averages that indicate system behavior changes.

```rust
let anomalies = detector.detect_timeseries_anomalies("cpu_usage", 12)?;
for anomaly in anomalies.iter().filter(|a| a.anomaly_type == AnomalyType::TrendAnomaly) {
    println!("Trend change detected: {}", anomaly.message);
}
```

#### Collective Anomaly
Finds correlated anomalies across multiple metrics.

```rust
let metrics = vec![
    "response_time".to_string(),
    "error_rate".to_string(),
    "cpu_usage".to_string(),
];

let collective = detector.detect_collective_anomalies(&metrics, 1)?;
for anomaly in collective {
    println!("Multiple metrics affected: {:?}", anomaly.context);
}
```

### Severity Levels

Anomalies are classified into four severity levels:

- **Low**: Minor deviation from baseline
- **Medium**: Noticeable deviation requiring attention
- **High**: Significant deviation requiring investigation
- **Critical**: Severe deviation requiring immediate action

### Integration with Chaos Scenarios

```rust
use mockforge_chaos::scenario_orchestrator::{ScenarioOrchestrator, ScenarioStep};

// Create orchestrator with anomaly detection
let detector = Arc::new(Mutex::new(AnomalyDetector::new(config)));

// In your scenario step
async fn chaos_step_with_detection(detector: Arc<Mutex<AnomalyDetector>>) {
    // Run chaos experiment
    inject_latency(100).await;

    // Collect metrics
    let response_time = measure_response_time().await;

    // Check for anomalies
    let mut det = detector.lock().unwrap();
    if let Some(anomaly) = det.detect_value_anomaly(
        "response_time",
        response_time,
        HashMap::new()
    ) {
        if anomaly.severity >= AnomalySeverity::High {
            // Alert or take corrective action
            alert_ops_team(anomaly).await;
        }
    }
}
```

---

## Chaos Mesh Integration

Chaos Mesh is a powerful cloud-native chaos engineering platform for Kubernetes. MockForge provides seamless integration with Chaos Mesh.

### Features

- **Pod Chaos**: Kill pods, fail containers, simulate pod failures
- **Network Chaos**: Inject latency, packet loss, bandwidth limits
- **Stress Chaos**: CPU and memory stress testing
- **IO Chaos**: Disk I/O failures and delays
- **Time Chaos**: Time skew and offset

### Prerequisites

1. Kubernetes cluster with Chaos Mesh installed
2. Kubernetes API access configured
3. Appropriate RBAC permissions

### Quick Start

```rust
use mockforge_chaos::{
    ChaosMeshClient, PodSelector, PodChaosAction, StressConfig
};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // Create Chaos Mesh client
    let client = ChaosMeshClient::new(
        "https://kubernetes.default.svc".to_string(),
        "default".to_string()
    );

    // Define target pods
    let selector = PodSelector {
        namespaces: vec!["production".to_string()],
        label_selectors: Some(HashMap::from([
            ("app".to_string(), "api-gateway".to_string()),
        ])),
        annotation_selectors: None,
        field_selectors: None,
        pod_phase_selectors: Some(vec!["Running".to_string()]),
    };

    // Create a pod chaos experiment
    let experiment = client.create_pod_chaos(
        "kill-api-gateway-pod",
        PodChaosAction::PodKill,
        selector,
        "one",              // Kill one pod
        Some("30s")         // Duration
    ).await.unwrap();

    println!("Created experiment: {}", experiment.metadata.name);
}
```

### Pod Chaos Examples

#### Kill Random Pod

```rust
use mockforge_chaos::{PodChaosAction, ExperimentType};

// Create experiment
let experiment = client.create_pod_chaos(
    "pod-kill-test",
    PodChaosAction::PodKill,
    selector.clone(),
    "random-max-percent",
    Some("5m")
).await?;

// Monitor status
let status = client.get_experiment_status(
    &ExperimentType::PodChaos,
    "pod-kill-test"
).await?;

println!("Experiment phase: {}", status.phase);
```

#### Container Kill

```rust
let experiment = client.create_pod_chaos(
    "container-kill-test",
    PodChaosAction::ContainerKill,
    selector,
    "all",
    Some("2m")
).await?;
```

### Network Chaos Examples

#### Inject Network Latency

```rust
let experiment = client.create_network_delay(
    "network-delay-test",
    selector,
    "100ms",           // Latency
    Some("10ms"),      // Jitter
    Some("5m")         // Duration
).await?;
```

#### Simulate Packet Loss

```rust
let experiment = client.create_network_loss(
    "packet-loss-test",
    selector,
    "10",              // 10% packet loss
    Some("3m")
).await?;
```

### Stress Chaos Examples

#### CPU Stress Test

```rust
let stress = StressConfig {
    cpu_workers: Some(4),
    cpu_load: Some(80),      // 80% load
    memory_workers: None,
    memory_size: None,
};

let experiment = client.create_stress_chaos(
    "cpu-stress-test",
    selector,
    stress,
    Some("10m")
).await?;
```

#### Memory Stress Test

```rust
let stress = StressConfig {
    cpu_workers: None,
    cpu_load: None,
    memory_workers: Some(2),
    memory_size: Some("512MB"),
};

let experiment = client.create_stress_chaos(
    "memory-stress-test",
    selector,
    stress,
    Some("5m")
).await?;
```

### Managing Experiments

#### Pause Experiment

```rust
client.pause_experiment(
    &ExperimentType::NetworkChaos,
    "network-delay-test"
).await?;
```

#### Resume Experiment

```rust
client.resume_experiment(
    &ExperimentType::NetworkChaos,
    "network-delay-test"
).await?;
```

#### Delete Experiment

```rust
client.delete_experiment(
    &ExperimentType::PodChaos,
    "pod-kill-test"
).await?;
```

#### List All Experiments

```rust
let experiments = client.list_experiments(
    &ExperimentType::NetworkChaos
).await?;

for exp in experiments {
    println!("{}: {}", exp.metadata.name, exp.spec.mode);
}
```

---

## Multi-Tenancy Support

MockForge provides comprehensive multi-tenancy support with resource isolation, quotas, and access controls.

### Features

- **Tenant Isolation**: Complete isolation between tenants
- **Resource Quotas**: Per-tenant limits on scenarios, executions, storage
- **Access Control**: Fine-grained permissions system
- **Usage Tracking**: Real-time resource usage monitoring
- **Plan-Based Features**: Different feature sets per tier

### Quick Start

```rust
use mockforge_chaos::{
    TenantManager, TenantPlan, ResourceQuota
};

// Create tenant manager
let manager = TenantManager::new();

// Create a new tenant
let tenant = manager.create_tenant(
    "acme-corp".to_string(),
    TenantPlan::Professional
).unwrap();

println!("Tenant ID: {}", tenant.id);
println!("Max scenarios: {}", tenant.quota.max_scenarios);
```

### Tenant Plans

MockForge supports four tenant plans:

#### Free Plan
```rust
let free_tenant = manager.create_tenant(
    "startup-co".to_string(),
    TenantPlan::Free
)?;

// Quotas:
// - 5 scenarios
// - 1 concurrent execution
// - 50 MB storage
// - Basic features only
```

#### Starter Plan
```rust
let starter_tenant = manager.create_tenant(
    "growing-business".to_string(),
    TenantPlan::Starter
)?;

// Quotas:
// - 20 scenarios
// - 5 concurrent executions
// - 500 MB storage
// - Observability features
```

#### Professional Plan
```rust
let pro_tenant = manager.create_tenant(
    "enterprise-team".to_string(),
    TenantPlan::Professional
)?;

// Quotas:
// - 100 scenarios
// - 20 concurrent executions
// - 5 GB storage
// - ML features, integrations
```

#### Enterprise Plan
```rust
let ent_tenant = manager.create_tenant(
    "large-corp".to_string(),
    TenantPlan::Enterprise
)?;

// Quotas:
// - Unlimited scenarios
// - 100 concurrent executions
// - 50 GB storage
// - All features
```

### Permission Checking

```rust
// Check if tenant can create scenarios
manager.check_permission(&tenant.id, "create_scenarios")?;

// Check ML features permission
if manager.check_permission(&tenant.id, "use_ml_features").is_ok() {
    // Enable ML-based anomaly detection
    enable_ml_features();
}
```

### Quota Management

#### Check Quota Before Action

```rust
// Check and increment quota atomically
manager.check_and_increment(&tenant.id, "scenario")?;

// Create scenario...

// Decrement when done
manager.decrement_usage(&tenant.id, "scenario")?;
```

#### Monitor Usage

```rust
let tenant = manager.get_tenant(&tenant_id)?;

println!("Scenarios: {}/{}",
    tenant.usage.scenarios,
    tenant.quota.max_scenarios
);

println!("Storage: {} MB / {} MB",
    tenant.usage.storage_mb,
    tenant.quota.max_storage_mb
);
```

### Rate Limiting

```rust
let mut tenant = manager.get_tenant(&tenant_id)?;

// Check rate limit (per minute)
tenant.check_rate_limit()?;

// Process request...
```

### Tenant Management

#### Upgrade Plan

```rust
manager.upgrade_plan(&tenant.id, TenantPlan::Enterprise)?;

let updated = manager.get_tenant(&tenant.id)?;
println!("New plan: {:?}", updated.plan);
```

#### Disable Tenant

```rust
manager.disable_tenant(&tenant.id)?;

// All requests will now be blocked
```

#### Enable Tenant

```rust
manager.enable_tenant(&tenant.id)?;
```

### Integration with Chaos Scenarios

```rust
async fn execute_chaos_scenario(
    manager: &TenantManager,
    tenant_id: &str,
    scenario_id: &str
) -> Result<()> {
    // Check permissions
    manager.check_permission(tenant_id, "execute_scenarios")?;

    // Check quota
    manager.check_and_increment(tenant_id, "execution")?;

    // Execute scenario
    let result = run_scenario(scenario_id).await;

    // Decrement execution counter
    manager.decrement_usage(tenant_id, "execution")?;

    result
}
```

---

## Custom Chaos Plugins

Extend MockForge with custom chaos engineering functionality through the plugin system.

### Features

- **Extensible Architecture**: Add custom fault injectors, metrics collectors, etc.
- **Lifecycle Hooks**: Before/after execution, error handling
- **Configuration Schema**: JSON Schema-based configuration
- **Capability Discovery**: Find plugins by capability
- **Hot Loading**: Load plugins at runtime

### Quick Start

```rust
use mockforge_chaos::{
    PluginRegistry, ChaosPlugin, PluginMetadata, PluginConfig,
    PluginContext, PluginResult, PluginCapability
};
use async_trait::async_trait;
use std::sync::Arc;

// Define your custom plugin
struct CustomPlugin {
    metadata: PluginMetadata,
}

#[async_trait]
impl ChaosPlugin for CustomPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::FaultInjection]
    }

    async fn initialize(&mut self, config: PluginConfig) -> Result<()> {
        // Initialize plugin
        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        // Custom logic here
        Ok(PluginResult::success(
            "Executed successfully".to_string(),
            HashMap::new()
        ))
    }

    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
}

// Register and use plugin
let registry = PluginRegistry::new();
let plugin = Arc::new(CustomPlugin { /* ... */ });

registry.register_plugin(plugin)?;
registry.execute_plugin("my-plugin", context).await?;
```

### Creating a Fault Injection Plugin

```rust
use serde_json::Value as JsonValue;

pub struct DatabaseFaultPlugin {
    metadata: PluginMetadata,
    config: Option<PluginConfig>,
}

impl DatabaseFaultPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "db-fault-injector".to_string(),
                name: "Database Fault Injector".to_string(),
                version: "1.0.0".to_string(),
                description: "Inject faults into database connections".to_string(),
                author: "Your Name".to_string(),
                homepage: None,
                repository: None,
                tags: vec!["database".to_string(), "fault".to_string()],
                dependencies: vec![],
                api_version: "v1".to_string(),
            },
            config: None,
        }
    }
}

#[async_trait]
impl ChaosPlugin for DatabaseFaultPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::FaultInjection]
    }

    async fn initialize(&mut self, config: PluginConfig) -> Result<()> {
        self.validate_config(&config)?;
        self.config = Some(config);
        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        let fault_type = context.parameters
            .get("fault_type")
            .and_then(|v| v.as_str())
            .unwrap_or("connection_timeout");

        match fault_type {
            "connection_timeout" => {
                // Simulate connection timeout
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
            "query_error" => {
                // Return database error
                return Ok(PluginResult::failure(
                    "Database query failed".to_string(),
                    "Simulated database error".to_string()
                ));
            }
            "slow_query" => {
                // Add latency to queries
                let latency_ms = context.parameters
                    .get("latency_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1000);

                tokio::time::sleep(
                    tokio::time::Duration::from_millis(latency_ms)
                ).await;
            }
            _ => {}
        }

        let mut data = HashMap::new();
        data.insert(
            "fault_type".to_string(),
            JsonValue::String(fault_type.to_string())
        );

        Ok(PluginResult::success(
            format!("Injected {} fault", fault_type),
            data
        ))
    }

    async fn cleanup(&mut self) -> Result<()> {
        self.config = None;
        Ok(())
    }

    fn config_schema(&self) -> Option<JsonValue> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean" },
                "config": {
                    "type": "object",
                    "properties": {
                        "default_timeout_ms": {
                            "type": "integer",
                            "default": 5000
                        }
                    }
                }
            }
        }))
    }
}
```

### Creating a Metrics Plugin

```rust
pub struct PrometheusMetricsPlugin {
    metadata: PluginMetadata,
    metrics: Arc<RwLock<Vec<HashMap<String, JsonValue>>>>,
}

impl PrometheusMetricsPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "prometheus-metrics".to_string(),
                name: "Prometheus Metrics Exporter".to_string(),
                version: "1.0.0".to_string(),
                description: "Export chaos metrics to Prometheus".to_string(),
                author: "Your Name".to_string(),
                homepage: None,
                repository: None,
                tags: vec!["metrics".to_string(), "prometheus".to_string()],
                dependencies: vec![],
                api_version: "v1".to_string(),
            },
            metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn export_metrics(&self) -> String {
        let metrics = self.metrics.read().unwrap();
        // Convert to Prometheus format
        format!("# Exported {} metrics", metrics.len())
    }
}

#[async_trait]
impl ChaosPlugin for PrometheusMetricsPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![
            PluginCapability::Metrics,
            PluginCapability::Observability
        ]
    }

    async fn initialize(&mut self, config: PluginConfig) -> Result<()> {
        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        let mut metric = HashMap::new();
        metric.insert(
            "timestamp".to_string(),
            JsonValue::String(chrono::Utc::now().to_rfc3339())
        );

        for (key, value) in context.parameters {
            metric.insert(key, value);
        }

        self.metrics.write().unwrap().push(metric.clone());

        Ok(PluginResult::success(
            "Metric recorded".to_string(),
            metric
        ))
    }

    async fn cleanup(&mut self) -> Result<()> {
        self.metrics.write().unwrap().clear();
        Ok(())
    }
}
```

### Plugin Hooks

```rust
use mockforge_chaos::PluginHook;

struct LoggingHook;

#[async_trait]
impl PluginHook for LoggingHook {
    async fn before_execute(&self, context: &PluginContext) -> Result<()> {
        println!("Executing plugin for tenant: {:?}", context.tenant_id);
        Ok(())
    }

    async fn after_execute(
        &self,
        context: &PluginContext,
        result: &PluginResult
    ) -> Result<()> {
        println!("Plugin execution result: {}", result.success);
        Ok(())
    }

    async fn on_error(
        &self,
        context: &PluginContext,
        error: &PluginError
    ) -> Result<()> {
        eprintln!("Plugin error: {}", error);
        Ok(())
    }
}

// Register hook
let registry = PluginRegistry::new();
registry.register_hook(Arc::new(LoggingHook));
```

### Finding Plugins by Capability

```rust
// Find all fault injection plugins
let fault_plugins = registry.find_by_capability(
    &PluginCapability::FaultInjection
);

for plugin in fault_plugins {
    println!("Found plugin: {} v{}", plugin.name, plugin.version);
}

// Find metrics plugins
let metrics_plugins = registry.find_by_capability(
    &PluginCapability::Metrics
);
```

### Complete Example

```rust
use mockforge_chaos::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create registry
    let registry = PluginRegistry::new();

    // Register plugins
    registry.register_plugin(Arc::new(DatabaseFaultPlugin::new()))?;
    registry.register_plugin(Arc::new(PrometheusMetricsPlugin::new()))?;

    // Register hooks
    registry.register_hook(Arc::new(LoggingHook));

    // Configure plugins
    let config = PluginConfig {
        enabled: true,
        config: HashMap::from([
            ("default_timeout_ms".to_string(), JsonValue::from(5000)),
        ]),
    };
    registry.configure_plugin("db-fault-injector", config)?;

    // Execute plugin
    let mut context = PluginContext::default();
    context.tenant_id = Some("tenant-123".to_string());
    context.parameters.insert(
        "fault_type".to_string(),
        JsonValue::String("slow_query".to_string())
    );

    let result = registry.execute_plugin("db-fault-injector", context).await?;
    println!("Result: {}", result.message);

    Ok(())
}
```

---

## Best Practices

### Anomaly Detection

1. **Collect Sufficient Baseline Data**: Ensure at least 30-50 samples before detecting anomalies
2. **Adjust Sensitivity**: Tune sensitivity based on your tolerance for false positives
3. **Monitor Multiple Metrics**: Use collective anomaly detection for correlated failures
4. **Set Appropriate Thresholds**: Different metrics may need different thresholds

### Chaos Mesh

1. **Start Small**: Begin with pod chaos on non-critical services
2. **Use Proper Selectors**: Carefully target pods to avoid unintended impact
3. **Set Duration Limits**: Always specify experiment duration
4. **Monitor Experiments**: Check experiment status regularly
5. **Clean Up**: Delete experiments when done

### Multi-Tenancy

1. **Plan Your Tiers**: Define clear feature boundaries between plans
2. **Monitor Usage**: Track resource usage to prevent quota exhaustion
3. **Implement Rate Limiting**: Protect shared resources
4. **Enforce Permissions**: Always check permissions before operations
5. **Handle Quota Errors**: Provide clear error messages when quotas are exceeded

### Custom Plugins

1. **Follow the Interface**: Implement all required trait methods
2. **Validate Configuration**: Check configuration in `validate_config`
3. **Handle Errors Gracefully**: Return meaningful error messages
4. **Document Your Plugin**: Provide clear metadata and config schema
5. **Test Thoroughly**: Write comprehensive tests for your plugins

---

## Troubleshooting

### Anomaly Detection

**Problem**: Too many false positives
- **Solution**: Increase `std_dev_threshold` or decrease `sensitivity`

**Problem**: Missing real anomalies
- **Solution**: Decrease `std_dev_threshold` or increase `sensitivity`

### Chaos Mesh

**Problem**: Experiments not starting
- **Solution**: Check RBAC permissions and namespace access

**Problem**: Can't delete experiments
- **Solution**: Verify Kubernetes API connectivity

### Multi-Tenancy

**Problem**: Quota exceeded errors
- **Solution**: Upgrade tenant plan or clean up unused resources

**Problem**: Permission denied
- **Solution**: Check tenant plan supports the requested feature

### Custom Plugins

**Problem**: Plugin not found
- **Solution**: Verify plugin is registered with correct ID

**Problem**: Configuration errors
- **Solution**: Validate config matches the schema

---

## Additional Resources

- [API Reference](./API_REFERENCE.md)
- [Chaos Engineering Best Practices](./CHAOS_ENGINEERING.md)
- [Observability Guide](./OBSERVABILITY.md)
- [Examples Directory](../examples/)
