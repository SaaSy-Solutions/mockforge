use mockforge_chaos::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Example demonstrating advanced ML features:
/// - Reinforcement Learning for remediation
/// - Multi-Armed Bandits for configuration optimization
/// - Predictive remediation for proactive failure prevention
/// - Advanced integrations for notifications
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("MockForge Advanced ML Features Demo\n");

    // ========================================
    // 1. REINFORCEMENT LEARNING
    // ========================================
    println!("=== Reinforcement Learning Demo ===\n");

    let rl_config = QLearningConfig {
        learning_rate: 0.1,
        discount_factor: 0.95,
        exploration_rate: 1.0,
        exploration_decay: 0.995,
        min_exploration: 0.01,
    };

    let mut rl_agent = RLAgent::new(rl_config);

    // Simulate learning cycle
    println!("Training RL agent over 100 episodes...");
    for episode in 0..100 {
        let state = generate_random_state();
        let action = rl_agent.select_action(&state).await;

        // Simulate taking action
        let (next_state, reward) = simulate_remediation(&state, &action);

        // Update Q-table
        rl_agent.update(&state, &action, reward, &next_state).await;

        if episode % 20 == 0 {
            let stats = rl_agent.get_stats().await;
            println!(
                "Episode {}: Q-table size = {}, Epsilon = {:.3}",
                episode,
                stats.get("q_table_size").unwrap(),
                stats.get("epsilon").unwrap()
            );
        }
    }

    println!("\nRL Agent trained!\n");

    // Test learned policy
    let test_state = SystemState {
        error_rate: 75,
        latency_level: 80,
        cpu_usage: 90,
        memory_usage: 85,
        active_failures: 5,
        service_health: "critical".to_string(),
    };

    let recommended_action = rl_agent.select_action(&test_state).await;
    println!("For critical state, RL recommends: {:?}\n", recommended_action);

    // Adaptive Risk Assessment
    let rl_agent_arc = Arc::new(RwLock::new(rl_agent));
    let risk_assessor = AdaptiveRiskAssessor::new(rl_agent_arc.clone());

    let risk = risk_assessor.assess_risk(&test_state).await;
    println!("Risk Assessment:");
    println!("  Level: {:.2}", risk.risk_level);
    println!("  Confidence: {:.2}", risk.confidence);
    println!("  Actions: {:?}\n", risk.recommended_actions);

    // ========================================
    // 2. MULTI-ARMED BANDITS
    // ========================================
    println!("=== Multi-Armed Bandit Demo ===\n");

    // Define different configurations to test
    let arms = vec![
        Arm::new(
            "conservative".to_string(),
            "Conservative Config".to_string(),
            serde_json::json!({
                "timeout": 5000,
                "retries": 3,
                "circuit_breaker_threshold": 0.5
            }),
        ),
        Arm::new(
            "moderate".to_string(),
            "Moderate Config".to_string(),
            serde_json::json!({
                "timeout": 10000,
                "retries": 5,
                "circuit_breaker_threshold": 0.3
            }),
        ),
        Arm::new(
            "aggressive".to_string(),
            "Aggressive Config".to_string(),
            serde_json::json!({
                "timeout": 15000,
                "retries": 7,
                "circuit_breaker_threshold": 0.2
            }),
        ),
        Arm::new(
            "adaptive".to_string(),
            "Adaptive Config".to_string(),
            serde_json::json!({
                "timeout": 8000,
                "retries": 4,
                "circuit_breaker_threshold": 0.4
            }),
        ),
    ];

    // Test different strategies
    for strategy_name in &["Thompson Sampling", "UCB1", "Epsilon-Greedy"] {
        println!("Testing {} strategy:", strategy_name);

        let strategy = match *strategy_name {
            "Thompson Sampling" => BanditStrategy::ThompsonSampling,
            "UCB1" => BanditStrategy::UCB1,
            _ => BanditStrategy::EpsilonGreedy { epsilon: 0.1 },
        };

        let bandit = MultiArmedBandit::new(arms.clone(), strategy);

        // Run experiment
        for _ in 0..200 {
            let arm_id = bandit.select_arm().await;
            let reward = simulate_config_performance(&arm_id);
            bandit.update(&arm_id, reward).await;
        }

        // Get results
        let report = bandit.get_report().await;
        println!("  Best arm: {:?}", report.best_arm);
        println!("  Total pulls: {}\n", report.total_pulls);

        for arm in &report.arms {
            println!(
                "  {}: {:.3} ({} pulls) - CI: [{:.3}, {:.3}]",
                arm.name,
                arm.mean_reward,
                arm.pulls,
                arm.confidence_interval.0,
                arm.confidence_interval.1
            );
        }
        println!();
    }

    // Traffic allocation example
    let best_bandit = MultiArmedBandit::new(
        arms.clone(),
        BanditStrategy::ThompsonSampling,
    );

    for _ in 0..100 {
        let arm_id = best_bandit.select_arm().await;
        let reward = simulate_config_performance(&arm_id);
        best_bandit.update(&arm_id, reward).await;
    }

    let bandit_arc = Arc::new(best_bandit);
    let allocator = TrafficAllocator::new(
        bandit_arc,
        std::time::Duration::from_secs(60),
    );

    let allocation = allocator.get_allocation().await;
    println!("Traffic Allocation:");
    for (arm_id, percentage) in allocation {
        println!("  {}: {:.1}%", arm_id, percentage * 100.0);
    }
    println!();

    // ========================================
    // 3. PREDICTIVE REMEDIATION
    // ========================================
    println!("=== Predictive Remediation Demo ===\n");

    let pred_engine = PredictiveRemediationEngine::new(10);

    // Simulate increasing error rate over time
    println!("Simulating degrading system metrics...");
    for i in 0..50 {
        let error_rate = 10.0 + (i as f64 * 1.5);
        let latency = 20.0 + (i as f64 * 1.2);
        let cpu = 30.0 + (i as f64 * 0.8);

        pred_engine
            .record(PredictiveMetricType::ErrorRate, error_rate)
            .await;
        pred_engine
            .record(PredictiveMetricType::Latency, latency)
            .await;
        pred_engine
            .record(PredictiveMetricType::CpuUsage, cpu)
            .await;
    }

    // Predict failures
    let predictions = pred_engine.predict_failures().await;

    println!("\nFailure Predictions:");
    for prediction in &predictions {
        println!("  Metric: {:?}", prediction.metric);
        println!("    Current: {:.2}", prediction.current_value);
        println!("    Predicted: {:.2}", prediction.predicted_value);
        println!("    Threshold: {:.2}", prediction.threshold);

        if let Some(ttf) = prediction.time_to_failure {
            println!(
                "    Time to failure: {} minutes",
                ttf.as_secs() / 60
            );
            println!("    Confidence: {:.2}", prediction.confidence);
            println!("    Recommended: {:?}", prediction.recommended_actions);
        } else {
            println!("    Status: Within normal range");
        }
        println!();
    }

    // Anomaly detection
    let anomalies = pred_engine.detect_anomalies().await;
    if !anomalies.is_empty() {
        println!("Anomalies detected:");
        for (metric, points) in anomalies {
            println!("  {:?}: {} anomalous points", metric, points.len());
        }
        println!();
    }

    // Trend analysis
    let pred_engine_arc = Arc::new(pred_engine);
    let trend_analyzer = TrendAnalyzer::new(pred_engine_arc.clone());
    let trend_report = trend_analyzer.analyze_trends().await;

    println!("Trend Analysis:");
    for (metric, trend) in trend_report.trends {
        println!("  {:?}:", metric);
        println!("    Direction: {:?}", trend.direction);
        println!("    Slope: {:.4}", trend.slope);
        println!("    Confidence: {:.2}", trend.confidence);
    }
    println!();

    // Proactive remediation
    let actions = pred_engine_arc.proactive_remediate().await;
    if !actions.is_empty() {
        println!("Proactive Remediation Actions:");
        for action in actions {
            println!("  - {:?}", action);
        }
        println!();
    }

    // ========================================
    // 4. INTEGRATIONS (Mock Example)
    // ========================================
    println!("=== Integration Demo ===\n");

    // Note: In production, you'd use real webhook URLs
    let integration_config = IntegrationConfig {
        slack: Some(SlackConfig {
            webhook_url: "https://hooks.slack.com/services/MOCK/WEBHOOK/URL".to_string(),
            channel: "#chaos-alerts".to_string(),
            username: Some("MockForge".to_string()),
            icon_emoji: Some(":robot_face:".to_string()),
            mention_users: vec![],
        }),
        teams: None,
        jira: None,
        pagerduty: None,
        grafana: None,
    };

    let manager = IntegrationManager::new(integration_config);

    let mut metadata = HashMap::new();
    metadata.insert("service".to_string(), serde_json::json!("api-gateway"));
    metadata.insert("error_rate".to_string(), serde_json::json!("75%"));
    metadata.insert("latency_p99".to_string(), serde_json::json!("2.5s"));

    let notification = Notification {
        title: "Predicted System Failure".to_string(),
        message: "ML models predict system failure in 5 minutes. Proactive remediation recommended.".to_string(),
        severity: NotificationSeverity::Critical,
        timestamp: chrono::Utc::now(),
        metadata,
    };

    println!("Notification would be sent to:");
    println!("  Title: {}", notification.title);
    println!("  Severity: {:?}", notification.severity);
    println!("  Message: {}", notification.message);
    println!();

    // In production:
    // let results = manager.notify(&notification).await?;

    // ========================================
    // 5. INTEGRATED EXAMPLE
    // ========================================
    println!("=== Integrated ML Pipeline ===\n");

    println!("1. Collecting system metrics...");
    let current_state = pred_engine_arc.get_system_state().await;
    println!("   State: {:?}", current_state);

    println!("\n2. Assessing risk with RL...");
    let risk = risk_assessor.assess_risk(&current_state).await;
    println!("   Risk level: {:.2}", risk.risk_level);

    println!("\n3. Checking failure predictions...");
    let predictions = pred_engine_arc.predict_failures().await;
    println!("   Active predictions: {}", predictions.len());

    println!("\n4. Selecting optimal configuration with MAB...");
    // Would select arm and get config in production

    println!("\n5. Decision:");
    if risk.risk_level > 0.7 || !predictions.is_empty() {
        println!("   → High risk or failure predicted");
        println!("   → Recommended actions: {:?}", risk.recommended_actions);
        println!("   → Sending notifications...");
        println!("   → Applying proactive remediation...");
    } else {
        println!("   → System healthy, continuing monitoring");
    }

    println!("\n=== Demo Complete ===");

    Ok(())
}

// Helper functions for simulation

fn generate_random_state() -> SystemState {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    SystemState {
        error_rate: rng.gen_range(0..100),
        latency_level: rng.gen_range(0..100),
        cpu_usage: rng.gen_range(0..100),
        memory_usage: rng.gen_range(0..100),
        active_failures: rng.gen_range(0..10),
        service_health: if rng.gen_bool(0.7) {
            "healthy".to_string()
        } else if rng.gen_bool(0.5) {
            "degraded".to_string()
        } else {
            "critical".to_string()
        },
    }
}

fn simulate_remediation(
    state: &SystemState,
    _action: &RLRemediationAction,
) -> (SystemState, f64) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Simulate action improving state
    let improvement = rng.gen_range(20..50);

    let next_state = SystemState {
        error_rate: state.error_rate.saturating_sub(improvement),
        latency_level: state.latency_level.saturating_sub(improvement / 2),
        cpu_usage: state.cpu_usage.saturating_sub(improvement / 3),
        memory_usage: state.memory_usage.saturating_sub(improvement / 4),
        active_failures: state.active_failures.saturating_sub(2),
        service_health: if state.error_rate < 30 {
            "healthy".to_string()
        } else if state.error_rate < 60 {
            "degraded".to_string()
        } else {
            "critical".to_string()
        },
    };

    // Calculate reward based on improvement
    let reward = improvement as f64;

    (next_state, reward)
}

fn simulate_config_performance(arm_id: &str) -> f64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Different configs have different success rates
    let base_rate = match arm_id {
        "conservative" => 0.75,
        "moderate" => 0.85,
        "aggressive" => 0.80,
        "adaptive" => 0.90,
        _ => 0.70,
    };

    // Add some noise
    let noise = rng.gen_range(-0.1..0.1);
    (base_rate + noise).max(0.0).min(1.0)
}
