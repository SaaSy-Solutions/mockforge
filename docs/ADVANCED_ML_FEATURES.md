# Advanced ML Features

This document covers the advanced machine learning and AI-powered features in MockForge.

## Table of Contents

1. [Reinforcement Learning](#reinforcement-learning)
2. [Multi-Armed Bandits](#multi-armed-bandits)
3. [Advanced Integrations](#advanced-integrations)
4. [Predictive Remediation](#predictive-remediation)

## Reinforcement Learning

MockForge uses Q-Learning to learn optimal remediation strategies based on system state and observed outcomes.

### Features

- **Self-Improving Recommendations**: The RL agent learns from experience which remediation actions work best
- **Adaptive Risk Assessment**: Risk levels are calculated based on learned Q-values and historical data
- **Exploration vs Exploitation**: Epsilon-greedy policy balances trying new actions vs using known good ones

### Usage

```rust
use mockforge_chaos::{RLAgent, QLearningConfig, SystemState, RLRemediationAction};

// Create RL agent with custom config
let config = QLearningConfig {
    learning_rate: 0.1,      // How quickly to learn (0.0 - 1.0)
    discount_factor: 0.95,   // Value of future rewards (0.0 - 1.0)
    exploration_rate: 1.0,   // Initial exploration (starts high)
    exploration_decay: 0.995,// Decay rate per update
    min_exploration: 0.01,   // Minimum exploration
};

let mut agent = RLAgent::new(config);

// Define current system state
let state = SystemState {
    error_rate: 50,           // 0-100
    latency_level: 60,        // 0-100
    cpu_usage: 80,           // 0-100
    memory_usage: 70,        // 0-100
    active_failures: 3,
    service_health: "degraded".to_string(),
};

// Select best action
let action = agent.select_action(&state).await;

// Execute action and observe results...
let next_state = SystemState {
    error_rate: 10,
    latency_level: 20,
    cpu_usage: 40,
    memory_usage: 50,
    active_failures: 0,
    service_health: "healthy".to_string(),
};

// Calculate reward
let reward = agent.calculate_reward(&state, &next_state);

// Update Q-table
agent.update(&state, &action, reward, &next_state).await;

// Save learned model
agent.save_model("rl_model.json").await?;
```

### Reward Function

The RL agent uses a multi-factor reward function:

- Error rate reduction: `+2.0` per percentage point
- Latency reduction: `+1.5` per percentage point
- CPU usage reduction: `+0.5` per percentage point
- Failure reduction: `+5.0` per failure resolved
- Health state improvements: up to `+50.0` for critical → healthy

### Adaptive Risk Assessment

```rust
use mockforge_chaos::AdaptiveRiskAssessor;
use std::sync::Arc;
use tokio::sync::RwLock;

let agent = Arc::new(RwLock::new(RLAgent::new(config)));
let assessor = AdaptiveRiskAssessor::new(agent);

// Assess current risk
let assessment = assessor.assess_risk(&state).await;

println!("Risk Level: {:.2}", assessment.risk_level);
println!("Confidence: {:.2}", assessment.confidence);
println!("Recommended Actions: {:?}", assessment.recommended_actions);

// Get risk trend over time
let trend = assessor.get_risk_trend().await;
```

## Multi-Armed Bandits

Beyond traditional A/B testing, MockForge supports multi-armed bandit algorithms for continuous optimization.

### Supported Algorithms

1. **Thompson Sampling**: Bayesian approach with Beta distributions
2. **UCB1 (Upper Confidence Bound)**: Optimistic exploration strategy
3. **Epsilon-Greedy**: Simple but effective exploration/exploitation balance

### Usage

```rust
use mockforge_chaos::{MultiArmedBandit, Arm, BanditStrategy};

// Define variants to test
let arms = vec![
    Arm::new(
        "v1".to_string(),
        "Original".to_string(),
        serde_json::json!({"timeout": 5000, "retries": 3}),
    ),
    Arm::new(
        "v2".to_string(),
        "Increased Timeout".to_string(),
        serde_json::json!({"timeout": 10000, "retries": 3}),
    ),
    Arm::new(
        "v3".to_string(),
        "More Retries".to_string(),
        serde_json::json!({"timeout": 5000, "retries": 5}),
    ),
    Arm::new(
        "v4".to_string(),
        "Both Increased".to_string(),
        serde_json::json!({"timeout": 10000, "retries": 5}),
    ),
];

// Create bandit with Thompson Sampling
let bandit = MultiArmedBandit::new(arms, BanditStrategy::ThompsonSampling);

// Or use UCB1
// let bandit = MultiArmedBandit::new(arms, BanditStrategy::UCB1);

// Or epsilon-greedy
// let bandit = MultiArmedBandit::new(arms, BanditStrategy::EpsilonGreedy { epsilon: 0.1 });

// Selection loop
for _ in 0..1000 {
    // Select arm based on strategy
    let arm_id = bandit.select_arm().await;

    // Get arm configuration
    let arm = bandit.get_arm(&arm_id).await.unwrap();

    // Execute with this configuration...
    let success = execute_with_config(&arm.config).await;

    // Update with reward (0.0 = failure, 1.0 = success)
    let reward = if success { 1.0 } else { 0.0 };
    bandit.update(&arm_id, reward).await;
}

// Get performance report
let report = bandit.get_report().await;
println!("Best arm: {:?}", report.best_arm);
println!("Total pulls: {}", report.total_pulls);

for arm in report.arms {
    println!(
        "{}: {:.3} success rate ({} pulls) - 95% CI: [{:.3}, {:.3}]",
        arm.name,
        arm.mean_reward,
        arm.pulls,
        arm.confidence_interval.0,
        arm.confidence_interval.1
    );
}
```

### Automatic Traffic Allocation

```rust
use mockforge_chaos::TrafficAllocator;
use std::sync::Arc;

let bandit = Arc::new(MultiArmedBandit::new(arms, BanditStrategy::ThompsonSampling));
let allocator = TrafficAllocator::new(bandit, std::time::Duration::from_secs(60));

// Get current allocation percentages
let allocation = allocator.get_allocation().await;

for (arm_id, percentage) in allocation {
    println!("{}: {:.1}% traffic", arm_id, percentage * 100.0);
}

// Start automatic reallocation
allocator.start_auto_allocation().await;
```

## Advanced Integrations

MockForge integrates with popular DevOps and monitoring tools.

### Supported Integrations

- **Slack**: Rich notifications with mentions
- **Microsoft Teams**: Adaptive cards with metadata
- **Jira**: Automatic ticket creation and updates
- **PagerDuty**: Incident triggering and resolution
- **Grafana**: Annotations and dashboard creation

### Configuration

```yaml
integrations:
  slack:
    webhook_url: "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
    channel: "#chaos-alerts"
    username: "MockForge"
    icon_emoji: ":robot_face:"
    mention_users:
      - "U12345678"  # User IDs to mention on critical alerts

  teams:
    webhook_url: "https://outlook.office.com/webhook/YOUR/WEBHOOK/URL"
    mention_users:
      - "user@company.com"

  jira:
    url: "https://your-company.atlassian.net"
    username: "bot@company.com"
    api_token: "your-api-token"
    project_key: "OPS"
    issue_type: "Incident"
    priority: "High"
    assignee: "oncall-team"

  pagerduty:
    routing_key: "your-routing-key"
    severity: "error"
    dedup_key_prefix: "mockforge"

  grafana:
    url: "https://grafana.company.com"
    api_key: "your-api-key"
    dashboard_uid: "chaos-dashboard"
    folder_uid: "chaos-folder"
```

### Usage

```rust
use mockforge_chaos::{
    IntegrationManager, IntegrationConfig, Notification, NotificationSeverity,
};
use std::collections::HashMap;

// Load configuration from file
let config: IntegrationConfig = serde_yaml::from_str(&config_yaml)?;

// Create integration manager
let manager = IntegrationManager::new(config);

// Send notification
let mut metadata = HashMap::new();
metadata.insert("service".to_string(), serde_json::json!("api-gateway"));
metadata.insert("region".to_string(), serde_json::json!("us-west-2"));
metadata.insert("error_rate".to_string(), serde_json::json!("45%"));

let notification = Notification {
    title: "High Error Rate Detected".to_string(),
    message: "The API gateway is experiencing a 45% error rate".to_string(),
    severity: NotificationSeverity::Critical,
    timestamp: chrono::Utc::now(),
    metadata,
};

// This will:
// - Send to Slack
// - Send to Teams
// - Create Jira ticket (for Error/Critical)
// - Trigger PagerDuty incident (for Critical)
// - Create Grafana annotation
let results = manager.notify(&notification).await?;

if results.slack_sent {
    println!("Sent to Slack");
}

if let Some(ticket) = results.jira_ticket {
    println!("Created Jira ticket: {}", ticket);
}

if let Some(incident) = results.pagerduty_incident {
    println!("Triggered PagerDuty incident: {}", incident);
}

for error in results.errors {
    eprintln!("Integration error: {}", error);
}
```

### Individual Integration Usage

```rust
use mockforge_chaos::{SlackNotifier, SlackConfig};

let config = SlackConfig {
    webhook_url: "https://hooks.slack.com/...".to_string(),
    channel: "#alerts".to_string(),
    username: Some("MockForge".to_string()),
    icon_emoji: Some(":robot_face:".to_string()),
    mention_users: vec!["U12345678".to_string()],
};

let notifier = SlackNotifier::new(config);
notifier.send(&notification).await?;
```

## Predictive Remediation

Predict failures before they occur and take proactive action.

### Features

- **Time Series Analysis**: Track metrics over time with moving averages and trends
- **Anomaly Detection**: Statistical detection using Z-score and IQR methods
- **Failure Prediction**: Linear regression and trend analysis
- **Proactive Remediation**: Automatic action before failures occur

### Usage

```rust
use mockforge_chaos::{
    PredictiveRemediationEngine, PredictiveMetricType, TrendAnalyzer,
};
use std::sync::Arc;

// Create prediction engine (predict 10 steps ahead)
let engine = PredictiveRemediationEngine::new(10);

// Record metrics over time
tokio::spawn({
    let engine = engine.clone();
    async move {
        loop {
            // Collect current metrics
            let error_rate = get_current_error_rate().await;
            let latency = get_current_latency().await;
            let cpu = get_current_cpu().await;

            engine.record(PredictiveMetricType::ErrorRate, error_rate).await;
            engine.record(PredictiveMetricType::Latency, latency).await;
            engine.record(PredictiveMetricType::CpuUsage, cpu).await;

            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }
});

// Get failure predictions
let predictions = engine.predict_failures().await;

for prediction in predictions {
    println!("Metric: {:?}", prediction.metric);
    println!("Current: {:.2}", prediction.current_value);
    println!("Predicted: {:.2}", prediction.predicted_value);

    if let Some(ttf) = prediction.time_to_failure {
        println!("Time to failure: {} seconds", ttf.as_secs());
        println!("Confidence: {:.2}", prediction.confidence);
        println!("Recommended actions: {:?}", prediction.recommended_actions);
    }
}

// Proactive remediation
let actions = engine.proactive_remediate().await;

for action in actions {
    println!("Applying proactive remediation: {:?}", action);
    apply_remediation(action).await?;
}
```

### Anomaly Detection

```rust
// Detect anomalies in all metrics
let anomalies = engine.detect_anomalies().await;

for (metric, anomaly_points) in anomalies {
    println!("Anomalies in {:?}:", metric);
    for (index, value) in anomaly_points {
        println!("  Index {}: {:.2}", index, value);
    }
}
```

### Trend Analysis

```rust
let engine = Arc::new(engine);
let analyzer = TrendAnalyzer::new(engine);

let trend_report = analyzer.analyze_trends().await;

for (metric, trend) in trend_report.trends {
    println!("{:?}:", metric);
    println!("  Direction: {:?}", trend.direction);
    println!("  Slope: {:.4}", trend.slope);
    println!("  Confidence: {:.2}", trend.confidence);
}
```

## Complete Example

```rust
use mockforge_chaos::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Setup Reinforcement Learning
    let rl_config = QLearningConfig::default();
    let rl_agent = Arc::new(RwLock::new(RLAgent::new(rl_config)));
    let risk_assessor = AdaptiveRiskAssessor::new(rl_agent.clone());

    // 2. Setup Multi-Armed Bandit
    let arms = vec![
        Arm::new("config_a".to_string(), "Config A".to_string(), serde_json::json!({})),
        Arm::new("config_b".to_string(), "Config B".to_string(), serde_json::json!({})),
        Arm::new("config_c".to_string(), "Config C".to_string(), serde_json::json!({})),
    ];
    let bandit = Arc::new(MultiArmedBandit::new(arms, BanditStrategy::ThompsonSampling));

    // 3. Setup Integrations
    let integration_config = IntegrationConfig {
        slack: Some(SlackConfig {
            webhook_url: std::env::var("SLACK_WEBHOOK")?,
            channel: "#chaos".to_string(),
            username: None,
            icon_emoji: None,
            mention_users: vec![],
        }),
        teams: None,
        jira: None,
        pagerduty: None,
        grafana: None,
    };
    let integrations = IntegrationManager::new(integration_config);

    // 4. Setup Predictive Remediation
    let pred_engine = Arc::new(PredictiveRemediationEngine::new(10));
    let trend_analyzer = TrendAnalyzer::new(pred_engine.clone());

    // 5. Main monitoring loop
    loop {
        // Get current system state
        let state = pred_engine.get_system_state().await;

        // Assess risk
        let risk = risk_assessor.assess_risk(&state).await;

        // Check predictions
        let predictions = pred_engine.predict_failures().await;

        // If high risk or imminent failure predicted
        if risk.risk_level > 0.7 || !predictions.is_empty() {
            // Send notifications
            let notification = Notification {
                title: "System Health Alert".to_string(),
                message: format!(
                    "Risk level: {:.2}, Predictions: {}",
                    risk.risk_level,
                    predictions.len()
                ),
                severity: NotificationSeverity::Warning,
                timestamp: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
            };

            integrations.notify(&notification).await?;

            // Get recommended action from RL agent
            let mut agent = rl_agent.write().await;
            let action = agent.select_action(&state).await;

            println!("Applying remediation: {:?}", action);

            // Apply action...

            // Observe result and update agent
            let next_state = pred_engine.get_system_state().await;
            let reward = agent.calculate_reward(&state, &next_state);
            agent.update(&state, &action, reward, &next_state).await;
        }

        // Analyze trends periodically
        let trends = trend_analyzer.analyze_trends().await;

        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}
```

## Best Practices

### Reinforcement Learning

1. **Start with exploration**: Begin with high epsilon (0.8-1.0) to explore action space
2. **Save models regularly**: Persist learned Q-tables to avoid losing knowledge
3. **Monitor Q-values**: Track average Q-values to ensure learning is occurring
4. **Tune learning rate**: Lower rates (0.01-0.1) for production, higher (0.3-0.5) for experimentation

### Multi-Armed Bandits

1. **Choose right strategy**:
   - Thompson Sampling: Best for most cases, Bayesian approach
   - UCB1: Good when you want optimistic exploration
   - Epsilon-Greedy: Simple and interpretable

2. **Allow exploration**: Don't switch to exploitation too quickly
3. **Monitor confidence intervals**: Wide intervals indicate need for more samples
4. **Consider context**: For context-dependent decisions, use contextual bandits

### Integrations

1. **Rate limiting**: Be mindful of API rate limits on external services
2. **Retry logic**: Implement retries for transient failures
3. **Failover**: Don't let integration failures block core functionality
4. **Security**: Use environment variables for sensitive credentials

### Predictive Remediation

1. **Collect enough data**: Need at least 100+ data points for reliable predictions
2. **Tune thresholds**: Adjust prediction thresholds based on your SLAs
3. **Combine methods**: Use both statistical and ML-based approaches
4. **Validate predictions**: Track prediction accuracy and adjust models
5. **Human in the loop**: For critical actions, require approval before execution

## API Reference

See the full API documentation with `cargo doc --open`
