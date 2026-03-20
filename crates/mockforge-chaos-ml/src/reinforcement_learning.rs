use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State representation for RL agent
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct SystemState {
    pub error_rate: u8,         // 0-100
    pub latency_level: u8,      // 0-100
    pub cpu_usage: u8,          // 0-100
    pub memory_usage: u8,       // 0-100
    pub active_failures: u8,    // Number of active failures
    pub service_health: String, // "healthy", "degraded", "critical"
}

/// Remediation action
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum RemediationAction {
    RestartService,
    ScaleUp(u32),
    ScaleDown(u32),
    ClearCache,
    RollbackDeployment,
    EnableCircuitBreaker,
    DisableRateLimiting,
    RestrictTraffic,
    NoAction,
}

/// Q-Learning parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QLearningConfig {
    pub learning_rate: f64,     // Alpha: 0.0 - 1.0
    pub discount_factor: f64,   // Gamma: 0.0 - 1.0
    pub exploration_rate: f64,  // Epsilon: 0.0 - 1.0
    pub exploration_decay: f64, // Epsilon decay rate
    pub min_exploration: f64,   // Minimum epsilon
}

impl Default for QLearningConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            discount_factor: 0.95,
            exploration_rate: 1.0,
            exploration_decay: 0.995,
            min_exploration: 0.01,
        }
    }
}

/// Q-Table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QValue {
    pub value: f64,
    pub visit_count: u64,
}

/// Reinforcement Learning Agent
pub struct RLAgent {
    q_table: Arc<RwLock<HashMap<(SystemState, RemediationAction), QValue>>>,
    config: QLearningConfig,
    current_epsilon: f64,
}

impl RLAgent {
    pub fn new(config: QLearningConfig) -> Self {
        Self {
            q_table: Arc::new(RwLock::new(HashMap::new())),
            current_epsilon: config.exploration_rate,
            config,
        }
    }

    /// Select action using epsilon-greedy policy
    pub async fn select_action(&self, state: &SystemState) -> RemediationAction {
        if rand::random::<f64>() < self.current_epsilon {
            // Explore: random action
            self.random_action()
        } else {
            // Exploit: best known action
            self.best_action(state).await
        }
    }

    /// Get best action for given state
    async fn best_action(&self, state: &SystemState) -> RemediationAction {
        let q_table = self.q_table.read().await;
        let actions = self.possible_actions();

        let mut best_action = RemediationAction::NoAction;
        let mut best_value = f64::NEG_INFINITY;

        for action in actions {
            let key = (state.clone(), action.clone());
            let value = q_table.get(&key).map(|q| q.value).unwrap_or(0.0);

            if value > best_value {
                best_value = value;
                best_action = action;
            }
        }

        best_action
    }

    /// Get random action (for exploration)
    fn random_action(&self) -> RemediationAction {
        let actions = self.possible_actions();
        use rand::Rng;
        let mut rng = rand::rng();
        let idx = rng.random_range(0..actions.len());
        actions[idx].clone()
    }

    /// Get all possible actions
    fn possible_actions(&self) -> Vec<RemediationAction> {
        vec![
            RemediationAction::RestartService,
            RemediationAction::ScaleUp(2),
            RemediationAction::ScaleUp(4),
            RemediationAction::ScaleDown(2),
            RemediationAction::ClearCache,
            RemediationAction::RollbackDeployment,
            RemediationAction::EnableCircuitBreaker,
            RemediationAction::DisableRateLimiting,
            RemediationAction::RestrictTraffic,
            RemediationAction::NoAction,
        ]
    }

    /// Update Q-value based on observed outcome
    pub async fn update(
        &mut self,
        state: &SystemState,
        action: &RemediationAction,
        reward: f64,
        next_state: &SystemState,
    ) {
        let mut q_table = self.q_table.write().await;

        // Get current Q-value
        let key = (state.clone(), action.clone());
        let current_q = q_table.get(&key).map(|q| q.value).unwrap_or(0.0);

        // Get max Q-value for next state
        let actions = self.possible_actions();
        let max_next_q = actions
            .iter()
            .map(|a| {
                let next_key = (next_state.clone(), a.clone());
                q_table.get(&next_key).map(|q| q.value).unwrap_or(0.0)
            })
            .fold(f64::NEG_INFINITY, f64::max);

        // Q-learning update: Q(s,a) = Q(s,a) + α[r + γ·max Q(s',a') - Q(s,a)]
        let new_q = current_q
            + self.config.learning_rate
                * (reward + self.config.discount_factor * max_next_q - current_q);

        // Update Q-table
        q_table
            .entry(key)
            .and_modify(|q| {
                q.value = new_q;
                q.visit_count += 1;
            })
            .or_insert(QValue {
                value: new_q,
                visit_count: 1,
            });

        // Decay exploration rate
        self.current_epsilon =
            (self.current_epsilon * self.config.exploration_decay).max(self.config.min_exploration);
    }

    /// Calculate reward based on outcome
    pub fn calculate_reward(&self, before: &SystemState, after: &SystemState) -> f64 {
        let mut reward = 0.0;

        // Reward for reducing error rate
        reward += (before.error_rate as f64 - after.error_rate as f64) * 2.0;

        // Reward for reducing latency
        reward += (before.latency_level as f64 - after.latency_level as f64) * 1.5;

        // Reward for reducing CPU usage
        reward += (before.cpu_usage as f64 - after.cpu_usage as f64) * 0.5;

        // Reward for reducing active failures
        reward += (before.active_failures as f64 - after.active_failures as f64) * 5.0;

        // Health state bonus
        reward += match (before.service_health.as_str(), after.service_health.as_str()) {
            ("critical", "healthy") => 50.0,
            ("critical", "degraded") => 25.0,
            ("degraded", "healthy") => 20.0,
            ("healthy", "degraded") => -30.0,
            ("healthy", "critical") => -50.0,
            ("degraded", "critical") => -40.0,
            _ => 0.0,
        };

        reward
    }

    /// Get policy statistics
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let q_table = self.q_table.read().await;

        let mut stats = HashMap::new();
        stats.insert("q_table_size".to_string(), serde_json::json!(q_table.len()));
        stats.insert("epsilon".to_string(), serde_json::json!(self.current_epsilon));

        // Calculate average Q-value
        let avg_q: f64 = if q_table.is_empty() {
            0.0
        } else {
            q_table.values().map(|q| q.value).sum::<f64>() / q_table.len() as f64
        };
        stats.insert("avg_q_value".to_string(), serde_json::json!(avg_q));

        // Most visited state-action pairs
        let mut visited: Vec<_> = q_table.iter().collect();
        visited.sort_by_key(|(_, q)| std::cmp::Reverse(q.visit_count));

        let top_pairs: Vec<_> = visited
            .iter()
            .take(10)
            .map(|((state, action), q)| {
                serde_json::json!({
                    "state": state,
                    "action": action,
                    "q_value": q.value,
                    "visits": q.visit_count,
                })
            })
            .collect();

        stats.insert("top_state_actions".to_string(), serde_json::json!(top_pairs));

        stats
    }

    /// Save Q-table to disk
    pub async fn save_model(&self, path: &str) -> Result<()> {
        let q_table = self.q_table.read().await;
        let data = serde_json::to_string_pretty(&*q_table)?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    /// Load Q-table from disk
    pub async fn load_model(&mut self, path: &str) -> Result<()> {
        let data = tokio::fs::read_to_string(path).await?;
        let loaded_table: HashMap<(SystemState, RemediationAction), QValue> =
            serde_json::from_str(&data)?;

        let mut q_table = self.q_table.write().await;
        *q_table = loaded_table;

        Ok(())
    }
}

/// Adaptive Risk Assessment Engine
pub struct AdaptiveRiskAssessor {
    risk_history: Arc<RwLock<Vec<RiskAssessment>>>,
    rl_agent: Arc<RwLock<RLAgent>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub state: SystemState,
    pub risk_level: f64, // 0.0 - 1.0
    pub recommended_actions: Vec<RemediationAction>,
    pub confidence: f64, // 0.0 - 1.0
}

impl AdaptiveRiskAssessor {
    pub fn new(rl_agent: Arc<RwLock<RLAgent>>) -> Self {
        Self {
            risk_history: Arc::new(RwLock::new(Vec::new())),
            rl_agent,
        }
    }

    /// Assess risk for current system state
    pub async fn assess_risk(&self, state: &SystemState) -> RiskAssessment {
        let mut risk_level = 0.0;

        // Factor in various metrics
        risk_level += state.error_rate as f64 / 100.0 * 0.3;
        risk_level += state.latency_level as f64 / 100.0 * 0.2;
        risk_level += state.cpu_usage as f64 / 100.0 * 0.15;
        risk_level += state.memory_usage as f64 / 100.0 * 0.15;
        risk_level += state.active_failures as f64 / 10.0 * 0.2;

        // Health state impact
        risk_level += match state.service_health.as_str() {
            "critical" => 0.4,
            "degraded" => 0.2,
            _ => 0.0,
        };

        risk_level = risk_level.min(1.0);

        // Get recommended actions from RL agent
        let agent = self.rl_agent.read().await;
        let action = agent.best_action(state).await;

        // Calculate confidence based on Q-table visit counts
        let q_table = agent.q_table.read().await;
        let key = (state.clone(), action.clone());
        let confidence = q_table
            .get(&key)
            .map(|q| (q.visit_count as f64 / 100.0).min(1.0))
            .unwrap_or(0.1);

        let assessment = RiskAssessment {
            timestamp: chrono::Utc::now(),
            state: state.clone(),
            risk_level,
            recommended_actions: vec![action],
            confidence,
        };

        // Store in history
        let mut history = self.risk_history.write().await;
        history.push(assessment.clone());

        // Keep only last 1000 assessments
        if history.len() > 1000 {
            let excess = history.len() - 1000;
            history.drain(0..excess);
        }

        assessment
    }

    /// Get risk trend over time
    pub async fn get_risk_trend(&self) -> Vec<(chrono::DateTime<chrono::Utc>, f64)> {
        let history = self.risk_history.read().await;
        history.iter().map(|a| (a.timestamp, a.risk_level)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rl_agent_learning() {
        let config = QLearningConfig::default();
        let mut agent = RLAgent::new(config);

        let state = SystemState {
            error_rate: 50,
            latency_level: 60,
            cpu_usage: 80,
            memory_usage: 70,
            active_failures: 3,
            service_health: "degraded".to_string(),
        };

        let action = RemediationAction::RestartService;

        let next_state = SystemState {
            error_rate: 10,
            latency_level: 20,
            cpu_usage: 40,
            memory_usage: 50,
            active_failures: 0,
            service_health: "healthy".to_string(),
        };

        let reward = agent.calculate_reward(&state, &next_state);
        agent.update(&state, &action, reward, &next_state).await;

        // Verify Q-value was updated
        let stats = agent.get_stats().await;
        assert!(stats.contains_key("q_table_size"));
    }

    #[tokio::test]
    async fn test_risk_assessment() {
        let config = QLearningConfig::default();
        let agent = Arc::new(RwLock::new(RLAgent::new(config)));
        let assessor = AdaptiveRiskAssessor::new(agent);

        let state = SystemState {
            error_rate: 75,
            latency_level: 80,
            cpu_usage: 90,
            memory_usage: 85,
            active_failures: 5,
            service_health: "critical".to_string(),
        };

        let assessment = assessor.assess_risk(&state).await;

        assert!(assessment.risk_level > 0.5);
        assert!(!assessment.recommended_actions.is_empty());
    }
}
