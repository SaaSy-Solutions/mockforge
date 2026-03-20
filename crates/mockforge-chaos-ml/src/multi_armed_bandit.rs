use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Arm (variant) in multi-armed bandit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arm {
    pub id: String,
    pub name: String,
    pub description: String,
    pub config: serde_json::Value,
    pub pulls: u64,
    pub total_reward: f64,
    pub mean_reward: f64,
}

impl Arm {
    pub fn new(id: String, name: String, config: serde_json::Value) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            config,
            pulls: 0,
            total_reward: 0.0,
            mean_reward: 0.0,
        }
    }

    pub fn update(&mut self, reward: f64) {
        self.pulls += 1;
        self.total_reward += reward;
        self.mean_reward = self.total_reward / self.pulls as f64;
    }
}

/// Thompson Sampling strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThompsonSampling {
    pub arms: HashMap<String, BetaDistribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaDistribution {
    pub alpha: f64, // Successes
    pub beta: f64,  // Failures
}

impl ThompsonSampling {
    pub fn new(arm_ids: &[String]) -> Self {
        let mut arms = HashMap::new();
        for id in arm_ids {
            arms.insert(
                id.clone(),
                BetaDistribution {
                    alpha: 1.0,
                    beta: 1.0,
                },
            );
        }
        Self { arms }
    }

    pub fn select_arm(&self) -> String {
        let mut best_arm = String::new();
        let mut best_sample = f64::NEG_INFINITY;

        for (arm_id, dist) in &self.arms {
            let sample = self.sample_beta(dist.alpha, dist.beta);
            if sample > best_sample {
                best_sample = sample;
                best_arm = arm_id.clone();
            }
        }

        best_arm
    }

    pub fn update(&mut self, arm_id: &str, reward: f64) {
        if let Some(dist) = self.arms.get_mut(arm_id) {
            if reward > 0.5 {
                dist.alpha += 1.0;
            } else {
                dist.beta += 1.0;
            }
        }
    }

    // Simple beta distribution sampling using gamma distributions
    fn sample_beta(&self, alpha: f64, beta: f64) -> f64 {
        let x = self.sample_gamma(alpha, 1.0);
        let y = self.sample_gamma(beta, 1.0);
        x / (x + y)
    }

    // Marsaglia and Tsang's method for gamma distribution
    fn sample_gamma(&self, shape: f64, scale: f64) -> f64 {
        if shape < 1.0 {
            return self.sample_gamma(shape + 1.0, scale) * rand::random::<f64>().powf(1.0 / shape);
        }

        let d = shape - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();

        loop {
            let x = self.sample_normal();
            let v = (1.0 + c * x).powi(3);

            if v > 0.0 {
                let u = rand::random::<f64>();
                if u < 1.0 - 0.0331 * x.powi(4) {
                    return d * v * scale;
                }
                if u.ln() < 0.5 * x.powi(2) + d * (1.0 - v + v.ln()) {
                    return d * v * scale;
                }
            }
        }
    }

    fn sample_normal(&self) -> f64 {
        // Box-Muller transform
        let u1 = rand::random::<f64>();
        let u2 = rand::random::<f64>();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// UCB1 (Upper Confidence Bound) strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UCB1 {
    pub arms: HashMap<String, ArmStats>,
    pub total_pulls: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmStats {
    pub pulls: u64,
    pub total_reward: f64,
    pub mean_reward: f64,
}

impl UCB1 {
    pub fn new(arm_ids: &[String]) -> Self {
        let mut arms = HashMap::new();
        for id in arm_ids {
            arms.insert(
                id.clone(),
                ArmStats {
                    pulls: 0,
                    total_reward: 0.0,
                    mean_reward: 0.0,
                },
            );
        }
        Self {
            arms,
            total_pulls: 0,
        }
    }

    pub fn select_arm(&self) -> String {
        // First pull all arms at least once
        for (arm_id, stats) in &self.arms {
            if stats.pulls == 0 {
                return arm_id.clone();
            }
        }

        // Calculate UCB for each arm
        let mut best_arm = String::new();
        let mut best_ucb = f64::NEG_INFINITY;

        for (arm_id, stats) in &self.arms {
            let ucb = stats.mean_reward
                + (2.0 * (self.total_pulls as f64).ln() / stats.pulls as f64).sqrt();

            if ucb > best_ucb {
                best_ucb = ucb;
                best_arm = arm_id.clone();
            }
        }

        best_arm
    }

    pub fn update(&mut self, arm_id: &str, reward: f64) {
        self.total_pulls += 1;

        if let Some(stats) = self.arms.get_mut(arm_id) {
            stats.pulls += 1;
            stats.total_reward += reward;
            stats.mean_reward = stats.total_reward / stats.pulls as f64;
        }
    }
}

/// Strategy for selecting arms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BanditStrategy {
    ThompsonSampling,
    UCB1,
    EpsilonGreedy { epsilon: f64 },
}

/// Multi-Armed Bandit for A/B/C/D/... testing
pub struct MultiArmedBandit {
    arms: Arc<RwLock<HashMap<String, Arm>>>,
    strategy: BanditStrategy,
    thompson_sampling: Arc<RwLock<Option<ThompsonSampling>>>,
    ucb1: Arc<RwLock<Option<UCB1>>>,
    epsilon: f64,
}

impl MultiArmedBandit {
    pub fn new(arms: Vec<Arm>, strategy: BanditStrategy) -> Self {
        let arm_ids: Vec<String> = arms.iter().map(|a| a.id.clone()).collect();

        let (thompson_sampling, ucb1, epsilon) = match &strategy {
            BanditStrategy::ThompsonSampling => (Some(ThompsonSampling::new(&arm_ids)), None, 0.0),
            BanditStrategy::UCB1 => (None, Some(UCB1::new(&arm_ids)), 0.0),
            BanditStrategy::EpsilonGreedy { epsilon } => (None, None, *epsilon),
        };

        let arms_map: HashMap<String, Arm> = arms.into_iter().map(|a| (a.id.clone(), a)).collect();

        Self {
            arms: Arc::new(RwLock::new(arms_map)),
            strategy,
            thompson_sampling: Arc::new(RwLock::new(thompson_sampling)),
            ucb1: Arc::new(RwLock::new(ucb1)),
            epsilon,
        }
    }

    /// Select arm based on strategy
    pub async fn select_arm(&self) -> String {
        match &self.strategy {
            BanditStrategy::ThompsonSampling => {
                let ts = self.thompson_sampling.read().await;
                ts.as_ref().unwrap().select_arm()
            }
            BanditStrategy::UCB1 => {
                let ucb = self.ucb1.read().await;
                ucb.as_ref().unwrap().select_arm()
            }
            BanditStrategy::EpsilonGreedy { .. } => {
                if rand::random::<f64>() < self.epsilon {
                    // Explore: random arm
                    self.random_arm().await
                } else {
                    // Exploit: best arm
                    self.best_arm().await
                }
            }
        }
    }

    async fn random_arm(&self) -> String {
        let arms = self.arms.read().await;
        let keys: Vec<_> = arms.keys().collect();
        if keys.is_empty() {
            return String::new();
        }
        use rand::Rng;
        let mut rng = rand::rng();
        let idx = rng.random_range(0..keys.len());
        keys[idx].clone()
    }

    async fn best_arm(&self) -> String {
        let arms = self.arms.read().await;
        let mut best_arm = String::new();
        let mut best_reward = f64::NEG_INFINITY;

        for (id, arm) in arms.iter() {
            if arm.mean_reward > best_reward {
                best_reward = arm.mean_reward;
                best_arm = id.clone();
            }
        }

        best_arm
    }

    /// Update arm with observed reward
    pub async fn update(&self, arm_id: &str, reward: f64) {
        // Update arm statistics
        {
            let mut arms = self.arms.write().await;
            if let Some(arm) = arms.get_mut(arm_id) {
                arm.update(reward);
            }
        }

        // Update strategy-specific state
        match &self.strategy {
            BanditStrategy::ThompsonSampling => {
                let mut ts = self.thompson_sampling.write().await;
                if let Some(ts) = ts.as_mut() {
                    ts.update(arm_id, reward);
                }
            }
            BanditStrategy::UCB1 => {
                let mut ucb = self.ucb1.write().await;
                if let Some(ucb) = ucb.as_mut() {
                    ucb.update(arm_id, reward);
                }
            }
            BanditStrategy::EpsilonGreedy { .. } => {
                // No additional state to update
            }
        }
    }

    /// Get arm by ID
    pub async fn get_arm(&self, arm_id: &str) -> Option<Arm> {
        let arms = self.arms.read().await;
        arms.get(arm_id).cloned()
    }

    /// Get all arms with statistics
    pub async fn get_all_arms(&self) -> Vec<Arm> {
        let arms = self.arms.read().await;
        arms.values().cloned().collect()
    }

    /// Get performance report
    pub async fn get_report(&self) -> BanditReport {
        let arms = self.arms.read().await;

        let mut arm_reports: Vec<_> = arms
            .values()
            .map(|arm| ArmReport {
                id: arm.id.clone(),
                name: arm.name.clone(),
                pulls: arm.pulls,
                mean_reward: arm.mean_reward,
                total_reward: arm.total_reward,
                confidence_interval: self.calculate_confidence_interval(arm),
            })
            .collect();

        arm_reports.sort_by(|a, b| b.mean_reward.partial_cmp(&a.mean_reward).unwrap());

        let total_pulls: u64 = arms.values().map(|a| a.pulls).sum();
        let best_arm = arm_reports.first().map(|r| r.id.clone());

        BanditReport {
            total_pulls,
            arms: arm_reports,
            best_arm,
            strategy: format!("{:?}", self.strategy),
        }
    }

    fn calculate_confidence_interval(&self, arm: &Arm) -> (f64, f64) {
        if arm.pulls < 2 {
            return (0.0, 1.0);
        }

        // 95% confidence interval using normal approximation
        let z = 1.96; // 95% confidence
        let std_error = (arm.mean_reward * (1.0 - arm.mean_reward) / arm.pulls as f64).sqrt();
        let margin = z * std_error;

        ((arm.mean_reward - margin).max(0.0), (arm.mean_reward + margin).min(1.0))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanditReport {
    pub total_pulls: u64,
    pub arms: Vec<ArmReport>,
    pub best_arm: Option<String>,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmReport {
    pub id: String,
    pub name: String,
    pub pulls: u64,
    pub mean_reward: f64,
    pub total_reward: f64,
    pub confidence_interval: (f64, f64),
}

/// Automatic traffic allocator
pub struct TrafficAllocator {
    bandit: Arc<MultiArmedBandit>,
    update_interval: std::time::Duration,
    min_samples: u64,
}

impl TrafficAllocator {
    pub fn new(bandit: Arc<MultiArmedBandit>, update_interval: std::time::Duration) -> Self {
        Self {
            bandit,
            update_interval,
            min_samples: 100,
        }
    }

    /// Get traffic allocation percentages
    pub async fn get_allocation(&self) -> HashMap<String, f64> {
        let arms = self.bandit.get_all_arms().await;
        let total_pulls: u64 = arms.iter().map(|a| a.pulls).sum();

        if total_pulls < self.min_samples {
            // Equal allocation during exploration phase
            let equal_share = 1.0 / arms.len() as f64;
            return arms.iter().map(|a| (a.id.clone(), equal_share)).collect();
        }

        // Allocate based on performance
        let total_reward: f64 = arms.iter().map(|a| a.mean_reward).sum();

        if total_reward == 0.0 {
            let equal_share = 1.0 / arms.len() as f64;
            return arms.iter().map(|a| (a.id.clone(), equal_share)).collect();
        }

        arms.iter()
            .map(|a| {
                let allocation = a.mean_reward / total_reward;
                (a.id.clone(), allocation)
            })
            .collect()
    }

    /// Start automatic reallocation
    pub async fn start_auto_allocation(&self) {
        let _bandit = self.bandit.clone();
        let interval = self.update_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                // Allocation is recalculated on-demand via get_allocation()
                // This task can trigger webhooks or notifications when allocation changes significantly
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_thompson_sampling() {
        let arms = vec![
            Arm::new("v1".to_string(), "Variant 1".to_string(), serde_json::json!({})),
            Arm::new("v2".to_string(), "Variant 2".to_string(), serde_json::json!({})),
            Arm::new("v3".to_string(), "Variant 3".to_string(), serde_json::json!({})),
        ];

        let bandit = MultiArmedBandit::new(arms, BanditStrategy::ThompsonSampling);

        // Simulate some pulls
        for _ in 0..100 {
            let arm_id = bandit.select_arm().await;
            let reward = if arm_id == "v2" { 0.8 } else { 0.3 };
            bandit.update(&arm_id, reward).await;
        }

        let report = bandit.get_report().await;
        assert_eq!(report.best_arm, Some("v2".to_string()));
    }

    #[tokio::test]
    async fn test_ucb1() {
        let arms = vec![
            Arm::new("a".to_string(), "Arm A".to_string(), serde_json::json!({})),
            Arm::new("b".to_string(), "Arm B".to_string(), serde_json::json!({})),
        ];

        let bandit = MultiArmedBandit::new(arms, BanditStrategy::UCB1);

        for _ in 0..50 {
            let arm_id = bandit.select_arm().await;
            let reward = if arm_id == "a" { 0.9 } else { 0.1 };
            bandit.update(&arm_id, reward).await;
        }

        let report = bandit.get_report().await;
        assert!(report.total_pulls > 0);
    }
}
