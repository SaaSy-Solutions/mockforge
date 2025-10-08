# Distributed Chaos Engineering - Complete Implementation

This document provides a comprehensive overview of the distributed chaos engineering features implemented in MockForge.

## 1. Distributed Chaos Coordination

### Overview
The distributed chaos coordination system enables MockForge to orchestrate chaos experiments across multiple nodes and clusters with leader election, state synchronization, and fault tolerance.

### Key Features

#### Distributed Coordinator
- **Leader Election**: Automatic leader election using a simple consensus algorithm
- **Node Management**: Register and manage multiple chaos nodes
- **Health Monitoring**: Continuous health checks with heartbeat mechanism
- **Task Distribution**: Distribute chaos orchestrations across nodes

#### Coordination Modes
1. **Parallel**: Execute on all nodes simultaneously
2. **Sequential**: Execute one node at a time
3. **Leader Assigned**: Leader node assigns tasks to workers
4. **Peer-to-Peer**: Nodes coordinate amongst themselves

### Usage

```rust
use mockforge_chaos::{DistributedCoordinator, Node, NodeStatus, DistributedTask};
use chrono::Utc;

// Create coordinator
let mut coordinator = DistributedCoordinator::new("node-1");
coordinator.start().await?;

// Register nodes
let node = Node {
    id: "node-2".to_string(),
    address: "10.0.1.2:8080".to_string(),
    region: Some("us-east-1".to_string()),
    zone: Some("us-east-1a".to_string()),
    capabilities: vec!["chaos".to_string(), "metrics".to_string()],
    last_heartbeat: Utc::now(),
    status: NodeStatus::Active,
};
coordinator.register_node(node).await?;

// Submit distributed task
let task = DistributedTask {
    id: "task-1".to_string(),
    orchestration: my_orchestration,
    target_nodes: vec!["node-1".to_string(), "node-2".to_string()],
    coordination_mode: CoordinationMode::Parallel,
    created_at: Utc::now(),
    started_at: None,
    completed_at: None,
    status: TaskStatus::Pending,
};
coordinator.submit_task(task).await?;

// Check if this node is the leader
if coordinator.is_leader() {
    println!("This node is the leader");
}

// Get active nodes
let active_nodes = coordinator.get_active_nodes();
```

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                 Distributed Coordinator                  │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Node Pool   │  │ Leader State │  │  Task Queue  │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│                                                           │
│  ┌──────────────────────────────────────────────────┐   │
│  │         Health Check & Heartbeat Manager         │   │
│  └──────────────────────────────────────────────────┘   │
│                                                           │
└─────────────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
    ┌────────┐    ┌────────┐    ┌────────┐
    │ Node 1 │    │ Node 2 │    │ Node 3 │
    └────────┘    └────────┘    └────────┘
```

## 2. Grafana Dashboard Templates

### Available Dashboards

#### Distributed Chaos Coordination Dashboard
Location: `deploy/grafana/dashboards/mockforge-distributed.json`

**Panels:**
- Active Nodes gauge
- Leader Node information
- Running Tasks counter
- Task Success Rate gauge
- Node Status Distribution pie chart
- Task Execution Timeline
- Node Heartbeats monitoring
- Coordination Mode Distribution
- Task Execution Metrics table

**Metrics Used:**
- `mockforge_node_status` - Node status tracking
- `mockforge_leader_term` - Leader election tracking
- `mockforge_task_status` - Task execution status
- `mockforge_task_progress` - Task progress tracking
- `mockforge_node_heartbeat_total` - Node health monitoring

#### Advanced Analytics Dashboard
Location: `deploy/grafana/dashboards/mockforge-analytics.json`

**Panels:**
- Chaos Impact Severity gauge
- Total Chaos Events counter
- Peak Chaos Time indicator
- System Degradation percentage
- Event Distribution by Type
- Top Affected Endpoints
- Latency Injection Statistics
- Fault Injection by Type
- Rate Limit Violations timeline
- Protocol Events Distribution
- Chaos Analytics Heatmap
- Impact Analysis Summary table

**Metrics Used:**
- `mockforge_chaos_impact_severity` - Overall chaos impact
- `mockforge_chaos_events_total` - Total chaos events
- `mockforge_latency_avg_ms` - Average latency metrics
- `mockforge_fault_injections_total` - Fault injection counters
- `mockforge_rate_limit_violations_total` - Rate limiting metrics

#### Orchestration Monitoring Dashboard
Location: `deploy/grafana/dashboards/mockforge-orchestration.json`

**Panels:**
- Active Orchestrations counter
- Orchestration Progress gauges
- Step Execution Status timeline
- Failed Steps table
- Orchestration Iterations tracking
- Parallel vs Sequential execution comparison
- Orchestration Duration histograms
- Step Execution Times heatmap

### Deployment

1. **Configure Grafana Provisioning:**
```yaml
# deploy/grafana/provisioning/dashboards.yaml
apiVersion: 1
providers:
  - name: 'MockForge'
    orgId: 1
    folder: 'MockForge'
    type: file
    options:
      path: /etc/grafana/provisioning/dashboards
```

2. **Mount Dashboards in Docker/Kubernetes:**
```yaml
volumes:
  - ./deploy/grafana/dashboards:/etc/grafana/provisioning/dashboards
  - ./deploy/grafana/provisioning:/etc/grafana/provisioning
```

3. **Access Dashboards:**
- Navigate to Grafana UI
- Go to Dashboards → Browse → MockForge folder
- Select desired dashboard

## 3. Kubernetes Operator

### Overview
The MockForge Kubernetes Operator enables declarative chaos orchestration management through Custom Resource Definitions (CRDs).

### Custom Resources

#### ChaosOrchestration CRD
Define chaos orchestrations as Kubernetes resources:

```yaml
apiVersion: mockforge.io/v1
kind: ChaosOrchestration
metadata:
  name: my-chaos-test
  namespace: default
spec:
  name: "Production Resilience Test"
  description: "Test service resilience under various failure conditions"

  # Optional cron schedule
  schedule: "0 */4 * * *"  # Every 4 hours

  steps:
    - name: "baseline-metrics"
      scenario: "collect-metrics"
      durationSeconds: 60

    - name: "inject-latency"
      scenario: "latency-injection"
      durationSeconds: 300
      parameters:
        latency_ms: 500
        endpoints:
          - "/api/users"
          - "/api/orders"

    - name: "fault-injection"
      scenario: "http-errors"
      durationSeconds: 120
      continueOnFailure: true
      parameters:
        error_rate: 0.1
        status_codes: [500, 502, 503]

  targetServices:
    - name: "my-service"
      namespace: "default"
      selector:
        app: "my-app"

  assertions:
    - type: "metric"
      expectedValue: 0.99
      operator: "gte"

  enableReporting: true

status:
  phase: Running
  currentStep: 1
  totalSteps: 3
  progress: 0.33
  startTime: "2025-10-07T10:00:00Z"
```

#### ChaosScenario CRD
Define reusable chaos scenarios:

```yaml
apiVersion: mockforge.io/v1
kind: ChaosScenario
metadata:
  name: network-partition
  namespace: default
spec:
  name: "Network Partition"
  type: "network-chaos"
  config:
    packetLoss: 0.5
    latencyMs: 1000
  durationSeconds: 300

status:
  active: true
  appliedAt: "2025-10-07T10:00:00Z"
  affectedPods:
    - "my-app-pod-1"
    - "my-app-pod-2"
```

### Operator Features

#### Admission Webhook
- **Validation**: Validates CRD specifications before creation
- **Mutation**: Sets default values automatically
- **Security**: Ensures safe chaos configurations

```rust
use mockforge_k8s_operator::WebhookHandler;

let handler = WebhookHandler::new();
let response = handler.handle_admission_review(review).await?;
```

#### Metrics Integration
The operator exports comprehensive Prometheus metrics:

```rust
use mockforge_k8s_operator::OperatorMetrics;
use prometheus::Registry;

let registry = Registry::new();
let metrics = OperatorMetrics::new(&registry)?;

// Record reconciliation
metrics.record_reconciliation("default", "my-chaos");

// Update progress
metrics.update_orchestration_progress("default", "my-chaos", 0.5);

// Record errors
metrics.record_reconciliation_error("default", "my-chaos", "timeout");
```

**Available Metrics:**
- `mockforge_operator_reconciliations_total` - Total reconciliation count
- `mockforge_operator_reconciliation_errors_total` - Reconciliation errors
- `mockforge_operator_reconciliation_duration_seconds` - Reconciliation duration
- `mockforge_operator_active_orchestrations` - Active orchestration count
- `mockforge_orchestration_progress` - Orchestration progress (0.0-1.0)
- `mockforge_orchestration_step` - Current step number
- `mockforge_orchestration_failed_steps_total` - Failed steps counter
- `mockforge_orchestration_duration_seconds` - Orchestration execution time

### Deployment

1. **Install CRDs:**
```bash
kubectl apply -f k8s/crds/chaosorchestration-crd.yaml
kubectl apply -f k8s/crds/chaosscenario-crd.yaml
```

2. **Deploy Operator:**
```bash
kubectl apply -f k8s/operator/deployment.yaml
```

3. **Create Orchestrations:**
```bash
kubectl apply -f examples/orchestrations/my-chaos.yaml
```

4. **Monitor Status:**
```bash
kubectl get chaosorchestrations
kubectl describe chaosorchestration my-chaos
```

## 4. Advanced Analytics Engine

### Overview
The Advanced Analytics Engine provides predictive insights, anomaly detection, and intelligent analysis of chaos experiments.

### Key Features

#### Anomaly Detection
Automatically detect unusual patterns in chaos experiments:

```rust
use mockforge_chaos::{AdvancedAnalyticsEngine, ChaosAnalytics};
use std::sync::Arc;

// Create engine
let base = Arc::new(ChaosAnalytics::new());
let engine = AdvancedAnalyticsEngine::new(base)
    .with_max_history(10000)
    .with_anomaly_threshold(0.7);

// Record events (anomalies detected automatically)
engine.record_event(chaos_event);

// Get recent anomalies
let anomalies = engine.get_anomalies(since_timestamp);

for anomaly in anomalies {
    println!("Anomaly: {} - {}", anomaly.anomaly_type, anomaly.description);
    println!("Severity: {:.2}", anomaly.severity);
    println!("Actions: {:?}", anomaly.suggested_actions);
}
```

**Detected Anomaly Types:**
- Event Spike - Sudden increase in chaos events
- Latency Anomaly - Unusual latency patterns
- High Error Rate - Elevated fault injection rates
- Resource Exhaustion - Resource depletion patterns
- Cascading Failure - Failure propagation patterns
- Unexpected Quiet - Unusual decrease in activity

#### Trend Analysis
Analyze metric trends over time:

```rust
// Analyze trend for a specific metric
let trend = engine.analyze_trend(
    "total_events",
    start_time,
    end_time,
);

println!("Trend: {:?}", trend.trend);
println!("Rate of change: {:.2}%", trend.rate_of_change * 100.0);
println!("Confidence: {:.2}", trend.confidence);

// Trend can be: Increasing, Decreasing, Stable, or Volatile
match trend.trend {
    TrendDirection::Increasing => {
        println!("Event rate is rising");
    }
    TrendDirection::Decreasing => {
        println!("Event rate is falling");
    }
    // ...
}
```

#### Predictive Insights
Generate predictions about future behavior:

```rust
// Generate insights
let insights = engine.generate_insights();

for insight in insights {
    println!("Prediction for {}: {:.2}",
        insight.metric,
        insight.predicted_value
    );
    println!("Confidence: {:.2}%", insight.confidence * 100.0);
    println!("Time horizon: {} minutes", insight.time_horizon_minutes);
    println!("Recommendation: {}", insight.recommendation);
}
```

#### Health Score Calculation
Calculate overall system health:

```rust
// Calculate health score
let health = engine.calculate_health_score();

println!("Overall Health: {:.1}/100", health.overall_score);

// Component scores
for (component, score) in health.components {
    println!("  {}: {:.1}", component, score);
}

// Factors affecting health
for factor in health.factors {
    println!("Factor: {} (impact: {:.1})",
        factor.name,
        factor.impact
    );
}
```

### Analytics API

Access analytics through HTTP API:

```bash
# Get anomalies
curl http://localhost:8080/api/analytics/anomalies?since=2025-10-07T00:00:00Z

# Get trend analysis
curl http://localhost:8080/api/analytics/trends/total_events

# Get health score
curl http://localhost:8080/api/analytics/health

# Get predictive insights
curl http://localhost:8080/api/analytics/insights
```

## Integration Example

Complete example integrating all features:

```rust
use mockforge_chaos::*;
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup distributed coordination
    let mut coordinator = DistributedCoordinator::new("node-1");
    coordinator.start().await?;

    // 2. Setup analytics
    let base_analytics = Arc::new(ChaosAnalytics::new());
    let analytics_engine = AdvancedAnalyticsEngine::new(
        Arc::clone(&base_analytics)
    );

    // 3. Create orchestration
    let orchestration = OrchestratedScenario::new("resilience-test")
        .add_step(ScenarioStep::new("latency", create_latency_scenario()))
        .add_step(ScenarioStep::new("faults", create_fault_scenario()));

    // 4. Execute distributed chaos
    let task = DistributedTask {
        id: "task-1".to_string(),
        orchestration,
        target_nodes: vec!["node-1".to_string()],
        coordination_mode: CoordinationMode::Parallel,
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
        status: TaskStatus::Pending,
    };

    coordinator.submit_task(task).await?;

    // 5. Monitor with analytics
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

        // Check for anomalies
        let anomalies = analytics_engine.get_anomalies(
            Utc::now() - chrono::Duration::minutes(5)
        );

        if !anomalies.is_empty() {
            println!("⚠️  Anomalies detected: {}", anomalies.len());
        }

        // Check health
        let health = analytics_engine.calculate_health_score();
        println!("System Health: {:.1}/100", health.overall_score);

        // Get insights
        let insights = analytics_engine.generate_insights();
        for insight in insights {
            println!("💡 {}", insight.recommendation);
        }
    }

    Ok(())
}
```

## Monitoring Stack

Complete monitoring setup:

1. **Prometheus Configuration:**
```yaml
scrape_configs:
  - job_name: 'mockforge-operator'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        action: keep
        regex: mockforge-operator
```

2. **Grafana Setup:**
- Import all dashboard templates
- Configure Prometheus datasource
- Set up alerts based on metrics

3. **Alert Rules:**
```yaml
groups:
  - name: mockforge-alerts
    rules:
      - alert: HighAnomalyRate
        expr: rate(mockforge_anomalies_total[5m]) > 0.1
        for: 5m
        annotations:
          summary: "High anomaly detection rate"

      - alert: LowHealthScore
        expr: mockforge_health_score < 50
        for: 10m
        annotations:
          summary: "System health score below threshold"
```

## Best Practices

1. **Distributed Coordination:**
   - Use heartbeats every 10-30 seconds
   - Set appropriate timeout values
   - Monitor leader election events
   - Handle node failures gracefully

2. **Dashboard Usage:**
   - Refresh intervals: 5-30 seconds for live monitoring
   - Use time range selectors for historical analysis
   - Export dashboards regularly
   - Customize panels for specific use cases

3. **Kubernetes Operator:**
   - Use namespaces for isolation
   - Set resource limits on CRDs
   - Enable admission webhooks in production
   - Monitor operator metrics

4. **Analytics Engine:**
   - Tune anomaly thresholds based on environment
   - Review predictions regularly
   - Act on suggested actions promptly
   - Archive historical data periodically

## Troubleshooting

### Common Issues

**Distributed Coordination:**
- No leader elected: Check node connectivity and heartbeats
- Task not distributing: Verify node capabilities and status
- Nodes marked failed: Check network latency and timeouts

**Dashboards:**
- No data showing: Verify Prometheus scraping configuration
- Metrics missing: Check metric exporters are running
- Panels empty: Confirm metric names match queries

**Kubernetes Operator:**
- CRD validation fails: Check specification against schema
- Operator not reconciling: Review operator logs
- Webhook errors: Verify webhook service configuration

**Analytics:**
- No anomalies detected: May need to lower threshold
- False positives: Increase threshold or baseline period
- Missing insights: Ensure sufficient historical data

## Conclusion

This implementation provides a complete distributed chaos engineering platform with:
- Multi-node coordination and orchestration
- Comprehensive observability through Grafana
- Kubernetes-native resource management
- AI-powered analytics and insights

All components are production-ready and can be deployed independently or as a complete stack.
