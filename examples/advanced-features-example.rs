//! Advanced Features Example
//!
//! Demonstrates ML-based anomaly detection, Chaos Mesh integration,
//! multi-tenancy, and custom plugins.

use mockforge_chaos::*;
use chrono::{Utc, Duration};
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value as JsonValue;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MockForge Advanced Features Demo ===\n");

    // 1. ML-Based Anomaly Detection
    println!("1. ML-Based Anomaly Detection");
    println!("-".repeat(50));
    anomaly_detection_demo()?;

    // 2. Chaos Mesh Integration
    println!("\n2. Chaos Mesh Integration");
    println!("-".repeat(50));
    chaos_mesh_demo().await?;

    // 3. Multi-Tenancy Support
    println!("\n3. Multi-Tenancy Support");
    println!("-".repeat(50));
    multi_tenancy_demo()?;

    // 4. Custom Plugins
    println!("\n4. Custom Plugins");
    println!("-".repeat(50));
    plugin_system_demo().await?;

    println!("\n=== Demo Complete ===");
    Ok(())
}

fn anomaly_detection_demo() -> Result<(), Box<dyn std::error::Error>> {
    // Configure anomaly detector
    let config = AnomalyDetectorConfig {
        std_dev_threshold: 3.0,
        min_baseline_samples: 20,
        moving_average_window: 5,
        enable_seasonal: false,
        seasonal_period: 24,
        sensitivity: 0.7,
    };

    let mut detector = AnomalyDetector::new(config);

    // Add baseline data (simulating normal response times)
    println!("Adding baseline data...");
    let base_time = Utc::now() - Duration::hours(24);
    for i in 0..50 {
        let point = TimeSeriesPoint {
            timestamp: base_time + Duration::minutes(i * 10),
            value: 100.0 + (i as f64 % 10.0) * 2.0, // Normal variation
            metadata: HashMap::new(),
        };
        detector.add_data_point("response_time_ms".to_string(), point);
    }

    // Update baseline
    let baseline = detector.update_baseline("response_time_ms")?;
    println!("Baseline established:");
    println!("  Mean: {:.2} ms", baseline.mean);
    println!("  Std Dev: {:.2} ms", baseline.std_dev);
    println!("  P95: {:.2} ms", baseline.p95);
    println!("  P99: {:.2} ms", baseline.p99);

    // Test normal value
    println!("\nTesting normal value (105 ms)...");
    let normal_result = detector.detect_value_anomaly(
        "response_time_ms",
        105.0,
        HashMap::new()
    );
    match normal_result {
        Some(anomaly) => println!("  Unexpected anomaly: {:?}", anomaly.severity),
        None => println!("  ✓ No anomaly detected (expected)"),
    }

    // Test anomalous value
    println!("\nTesting anomalous value (500 ms)...");
    let mut context = HashMap::new();
    context.insert("endpoint".to_string(), "/api/users".to_string());

    let anomaly_result = detector.detect_value_anomaly(
        "response_time_ms",
        500.0,
        context
    );

    match anomaly_result {
        Some(anomaly) => {
            println!("  ✓ Anomaly detected:");
            println!("    Type: {:?}", anomaly.anomaly_type);
            println!("    Severity: {:?}", anomaly.severity);
            println!("    Expected range: {:.2} - {:.2} ms",
                anomaly.expected_range.0,
                anomaly.expected_range.1
            );
            println!("    Deviation score: {:.2}", anomaly.deviation_score);
        }
        None => println!("  Unexpected: No anomaly detected"),
    }

    // Detect time series anomalies
    println!("\nDetecting time series anomalies...");
    let ts_anomalies = detector.detect_timeseries_anomalies(
        "response_time_ms",
        24
    )?;
    println!("  Found {} anomalies in last 24 hours", ts_anomalies.len());

    Ok(())
}

async fn chaos_mesh_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("Chaos Mesh Integration Demo");
    println!("Note: Requires Kubernetes cluster with Chaos Mesh installed\n");

    // Create Chaos Mesh client
    let client = ChaosMeshClient::new(
        "https://kubernetes.default.svc".to_string(),
        "default".to_string()
    );

    // Define pod selector
    let selector = PodSelector {
        namespaces: vec!["production".to_string()],
        label_selectors: Some(HashMap::from([
            ("app".to_string(), "demo-service".to_string()),
        ])),
        annotation_selectors: None,
        field_selectors: None,
        pod_phase_selectors: Some(vec!["Running".to_string()]),
    };

    println!("Example 1: Pod Kill Chaos");
    println!("Would create experiment to kill one random pod every 30s");
    println!("  Target: production/demo-service");
    println!("  Action: PodKill");
    println!("  Mode: one");
    println!("  Duration: 5m\n");

    // In real scenario:
    // let pod_chaos = client.create_pod_chaos(
    //     "demo-pod-kill",
    //     PodChaosAction::PodKill,
    //     selector.clone(),
    //     "one",
    //     Some("5m")
    // ).await?;

    println!("Example 2: Network Delay");
    println!("Would inject 100ms latency with 10ms jitter");
    println!("  Target: production/demo-service");
    println!("  Latency: 100ms ± 10ms");
    println!("  Duration: 3m\n");

    // In real scenario:
    // let network_chaos = client.create_network_delay(
    //     "demo-network-delay",
    //     selector.clone(),
    //     "100ms",
    //     Some("10ms"),
    //     Some("3m")
    // ).await?;

    println!("Example 3: CPU Stress");
    println!("Would stress CPU at 80% load");
    println!("  Target: production/demo-service");
    println!("  Workers: 4");
    println!("  Load: 80%");
    println!("  Duration: 10m\n");

    // In real scenario:
    // let stress = StressConfig {
    //     cpu_workers: Some(4),
    //     cpu_load: Some(80),
    //     memory_workers: None,
    //     memory_size: None,
    // };
    // let stress_chaos = client.create_stress_chaos(
    //     "demo-cpu-stress",
    //     selector,
    //     stress,
    //     Some("10m")
    // ).await?;

    println!("✓ Chaos Mesh examples prepared");
    Ok(())
}

fn multi_tenancy_demo() -> Result<(), Box<dyn std::error::Error>> {
    let manager = TenantManager::new();

    // Create tenants with different plans
    println!("Creating tenants...");

    let free_tenant = manager.create_tenant(
        "startup-labs".to_string(),
        TenantPlan::Free
    )?;
    println!("  ✓ Free tenant: {} (ID: {})", free_tenant.name, free_tenant.id);

    let pro_tenant = manager.create_tenant(
        "techcorp-inc".to_string(),
        TenantPlan::Professional
    )?;
    println!("  ✓ Pro tenant: {} (ID: {})", pro_tenant.name, pro_tenant.id);

    // Display quotas
    println!("\nTenant Quotas:");
    println!("  startup-labs (Free):");
    println!("    Max scenarios: {}", free_tenant.quota.max_scenarios);
    println!("    Max executions: {}", free_tenant.quota.max_concurrent_executions);
    println!("    ML features: {}", free_tenant.permissions.can_use_ml_features);

    println!("\n  techcorp-inc (Professional):");
    println!("    Max scenarios: {}", pro_tenant.quota.max_scenarios);
    println!("    Max executions: {}", pro_tenant.quota.max_concurrent_executions);
    println!("    ML features: {}", pro_tenant.permissions.can_use_ml_features);

    // Simulate resource usage
    println!("\nSimulating resource usage for startup-labs...");
    for i in 1..=3 {
        match manager.check_and_increment(&free_tenant.id, "scenario") {
            Ok(_) => println!("  ✓ Created scenario {}/{}", i, free_tenant.quota.max_scenarios),
            Err(e) => println!("  ✗ Failed: {}", e),
        }
    }

    let updated_tenant = manager.get_tenant(&free_tenant.id)?;
    println!("\nCurrent usage: {}/{} scenarios",
        updated_tenant.usage.scenarios,
        updated_tenant.quota.max_scenarios
    );

    // Attempt to exceed quota
    println!("\nAttempting to exceed quota...");
    for i in 4..=6 {
        match manager.check_and_increment(&free_tenant.id, "scenario") {
            Ok(_) => println!("  ✓ Created scenario {}", i),
            Err(MultiTenancyError::QuotaExceeded { tenant: _, quota_type }) => {
                println!("  ✗ Quota exceeded: {}", quota_type);
                break;
            }
            Err(e) => println!("  ✗ Error: {}", e),
        }
    }

    // Upgrade plan
    println!("\nUpgrading startup-labs to Starter plan...");
    manager.upgrade_plan(&free_tenant.id, TenantPlan::Starter)?;

    let upgraded_tenant = manager.get_tenant(&free_tenant.id)?;
    println!("  ✓ Upgraded successfully");
    println!("  New quota: {} scenarios", upgraded_tenant.quota.max_scenarios);

    // Permission checking
    println!("\nChecking permissions...");
    match manager.check_permission(&free_tenant.id, "use_ml_features") {
        Ok(_) => println!("  ✓ startup-labs can use ML features"),
        Err(_) => println!("  ✗ startup-labs cannot use ML features (Starter+ required)"),
    }

    match manager.check_permission(&pro_tenant.id, "use_ml_features") {
        Ok(_) => println!("  ✓ techcorp-inc can use ML features"),
        Err(_) => println!("  ✗ techcorp-inc cannot use ML features"),
    }

    Ok(())
}

async fn plugin_system_demo() -> Result<(), Box<dyn std::error::Error>> {
    let registry = PluginRegistry::new();

    // Register built-in plugins
    println!("Registering plugins...");
    registry.register_plugin(Arc::new(CustomFaultPlugin::new()))?;
    println!("  ✓ Custom Fault Injector");

    registry.register_plugin(Arc::new(MetricsPlugin::new()))?;
    println!("  ✓ Metrics Collector");

    // List all plugins
    println!("\nRegistered plugins:");
    for plugin in registry.list_plugins() {
        println!("  - {} v{}: {}", plugin.name, plugin.version, plugin.description);
    }

    // Configure plugins
    println!("\nConfiguring plugins...");
    let mut fault_config = PluginConfig::default();
    fault_config.config.insert(
        "fault_probability".to_string(),
        JsonValue::from(0.3)
    );
    registry.configure_plugin("custom-fault-injector", fault_config)?;
    println!("  ✓ Configured custom-fault-injector");

    registry.configure_plugin("metrics-collector", PluginConfig::default())?;
    println!("  ✓ Configured metrics-collector");

    // Find plugins by capability
    println!("\nFinding fault injection plugins...");
    let fault_plugins = registry.find_by_capability(&PluginCapability::FaultInjection);
    for plugin in fault_plugins {
        println!("  - {}", plugin.name);
    }

    // Execute fault injection plugin
    println!("\nExecuting custom fault injector...");
    let mut context = PluginContext::default();
    context.tenant_id = Some("demo-tenant".to_string());
    context.scenario_id = Some("scenario-123".to_string());
    context.parameters.insert(
        "fault_type".to_string(),
        JsonValue::String("timeout".to_string())
    );

    let result = registry.execute_plugin("custom-fault-injector", context).await?;
    println!("  Result: {}", result.message);
    println!("  Success: {}", result.success);
    println!("  Data: {:?}", result.data);

    // Execute metrics plugin
    println!("\nCollecting metrics...");
    let mut metrics_context = PluginContext::default();
    metrics_context.tenant_id = Some("demo-tenant".to_string());
    metrics_context.parameters.insert(
        "metric_name".to_string(),
        JsonValue::String("chaos_executions".to_string())
    );
    metrics_context.parameters.insert(
        "value".to_string(),
        JsonValue::from(42)
    );

    let metrics_result = registry.execute_plugin("metrics-collector", metrics_context).await?;
    println!("  {}", metrics_result.message);

    println!("\n✓ Plugin system demo complete");
    Ok(())
}
