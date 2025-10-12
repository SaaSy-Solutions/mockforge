//! Latency simulation and fault injection for MockForge

use crate::Result;
use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;

/// Latency distribution types
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LatencyDistribution {
    /// Fixed latency with optional jitter (backward compatible)
    #[default]
    Fixed,
    /// Normal (Gaussian) distribution
    Normal,
    /// Pareto (power-law) distribution for heavy-tailed latency
    Pareto,
}

/// Latency profile configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LatencyProfile {
    /// Base latency in milliseconds (mean for distributions)
    pub base_ms: u64,
    /// Random jitter range in milliseconds (for fixed distribution)
    pub jitter_ms: u64,
    /// Distribution type for latency variation
    #[serde(default)]
    pub distribution: LatencyDistribution,
    /// Standard deviation for normal distribution (in milliseconds)
    #[serde(default)]
    pub std_dev_ms: Option<f64>,
    /// Shape parameter for pareto distribution (alpha > 0)
    #[serde(default)]
    pub pareto_shape: Option<f64>,
    /// Minimum latency bound (prevents negative values)
    #[serde(default)]
    pub min_ms: u64,
    /// Maximum latency bound (prevents extreme values)
    #[serde(default)]
    pub max_ms: Option<u64>,
    /// Tag-based latency overrides
    pub tag_overrides: HashMap<String, u64>,
}

impl Default for LatencyProfile {
    fn default() -> Self {
        Self {
            base_ms: 50,   // 50ms base latency
            jitter_ms: 20, // ±20ms jitter
            distribution: LatencyDistribution::Fixed,
            std_dev_ms: None,
            pareto_shape: None,
            min_ms: 0,
            max_ms: None,
            tag_overrides: HashMap::new(),
        }
    }
}

impl LatencyProfile {
    /// Create a new latency profile with fixed distribution (backward compatible)
    pub fn new(base_ms: u64, jitter_ms: u64) -> Self {
        Self {
            base_ms,
            jitter_ms,
            distribution: LatencyDistribution::Fixed,
            std_dev_ms: None,
            pareto_shape: None,
            min_ms: 0,
            max_ms: None,
            tag_overrides: HashMap::new(),
        }
    }

    /// Create a new latency profile with normal distribution
    pub fn with_normal_distribution(base_ms: u64, std_dev_ms: f64) -> Self {
        Self {
            base_ms,
            jitter_ms: 0, // Not used for normal distribution
            distribution: LatencyDistribution::Normal,
            std_dev_ms: Some(std_dev_ms),
            pareto_shape: None,
            min_ms: 0,
            max_ms: None,
            tag_overrides: HashMap::new(),
        }
    }

    /// Create a new latency profile with pareto distribution
    pub fn with_pareto_distribution(base_ms: u64, shape: f64) -> Self {
        Self {
            base_ms,
            jitter_ms: 0, // Not used for pareto distribution
            distribution: LatencyDistribution::Pareto,
            std_dev_ms: None,
            pareto_shape: Some(shape),
            min_ms: 0,
            max_ms: None,
            tag_overrides: HashMap::new(),
        }
    }

    /// Add a tag-based latency override
    pub fn with_tag_override(mut self, tag: String, latency_ms: u64) -> Self {
        self.tag_overrides.insert(tag, latency_ms);
        self
    }

    /// Set minimum latency bound
    pub fn with_min_ms(mut self, min_ms: u64) -> Self {
        self.min_ms = min_ms;
        self
    }

    /// Set maximum latency bound
    pub fn with_max_ms(mut self, max_ms: u64) -> Self {
        self.max_ms = Some(max_ms);
        self
    }

    /// Calculate latency for a request with optional tags
    pub fn calculate_latency(&self, tags: &[String]) -> Duration {
        let mut rng = rand::rng();

        // Check for tag overrides (use the first matching tag)
        // Note: Tag overrides always use fixed latency for simplicity
        if let Some(&override_ms) = tags.iter().find_map(|tag| self.tag_overrides.get(tag)) {
            return Duration::from_millis(override_ms);
        }

        let mut latency_ms = match self.distribution {
            LatencyDistribution::Fixed => {
                // Original behavior: base + jitter
                let jitter = if self.jitter_ms > 0 {
                    rng.random_range(0..=self.jitter_ms * 2).saturating_sub(self.jitter_ms)
                } else {
                    0
                };
                self.base_ms.saturating_add(jitter)
            }
            LatencyDistribution::Normal => {
                // Simple approximation of normal distribution using Box-Muller transform
                let std_dev = self.std_dev_ms.unwrap_or((self.base_ms as f64) * 0.2);
                let mean = self.base_ms as f64;

                // Generate two uniform random numbers
                let u1: f64 = rng.random();
                let u2: f64 = rng.random();

                // Box-Muller transform
                let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                (mean + std_dev * z0).max(0.0) as u64
            }
            LatencyDistribution::Pareto => {
                // Pareto distribution: P(x) = shape * scale^shape / x^(shape+1) for x >= scale
                let shape = self.pareto_shape.unwrap_or(2.0);
                let scale = self.base_ms as f64;

                // Inverse CDF method for Pareto distribution
                let u: f64 = rng.random();
                (scale / (1.0 - u).powf(1.0 / shape)) as u64
            }
        };

        // Apply bounds
        latency_ms = latency_ms.max(self.min_ms);
        if let Some(max_ms) = self.max_ms {
            latency_ms = latency_ms.min(max_ms);
        }

        Duration::from_millis(latency_ms)
    }
}

/// Fault injection configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FaultConfig {
    /// Probability of failure (0.0 to 1.0)
    pub failure_rate: f64,
    /// HTTP status codes to return on failure
    pub status_codes: Vec<u16>,
    /// Custom error responses
    pub error_responses: HashMap<String, serde_json::Value>,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            failure_rate: 0.0,
            status_codes: vec![500, 502, 503, 504],
            error_responses: HashMap::new(),
        }
    }
}

impl FaultConfig {
    /// Create a new fault configuration
    pub fn new(failure_rate: f64) -> Self {
        Self {
            failure_rate: failure_rate.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Add a status code to the failure responses
    pub fn with_status_code(mut self, code: u16) -> Self {
        if !self.status_codes.contains(&code) {
            self.status_codes.push(code);
        }
        self
    }

    /// Add a custom error response
    pub fn with_error_response(mut self, key: String, response: serde_json::Value) -> Self {
        self.error_responses.insert(key, response);
        self
    }

    /// Determine if a failure should occur
    pub fn should_fail(&self) -> bool {
        if self.failure_rate <= 0.0 {
            return false;
        }
        if self.failure_rate >= 1.0 {
            return true;
        }

        let mut rng = rand::rng();
        rng.random_bool(self.failure_rate)
    }

    /// Get a random failure response
    pub fn get_failure_response(&self) -> (u16, Option<serde_json::Value>) {
        let mut rng = rand::rng();

        let status_code = if self.status_codes.is_empty() {
            500
        } else {
            let index = rng.random_range(0..self.status_codes.len());
            self.status_codes[index]
        };

        let error_response = if self.error_responses.is_empty() {
            None
        } else {
            let keys: Vec<&String> = self.error_responses.keys().collect();
            let key = keys[rng.random_range(0..keys.len())];
            self.error_responses.get(key).cloned()
        };

        (status_code, error_response)
    }
}

/// Latency and fault injector
#[derive(Debug, Clone)]
pub struct LatencyInjector {
    /// Latency profile
    latency_profile: LatencyProfile,
    /// Fault configuration
    fault_config: FaultConfig,
    /// Whether injection is enabled
    enabled: bool,
}

impl LatencyInjector {
    /// Create a new latency injector
    pub fn new(latency_profile: LatencyProfile, fault_config: FaultConfig) -> Self {
        Self {
            latency_profile,
            fault_config,
            enabled: true,
        }
    }

    /// Enable or disable injection
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if injection is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Inject latency for a request
    pub async fn inject_latency(&self, tags: &[String]) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let latency = self.latency_profile.calculate_latency(tags);
        if !latency.is_zero() {
            tokio::time::sleep(latency).await;
        }

        Ok(())
    }

    /// Check if a failure should be injected
    pub fn should_inject_failure(&self) -> bool {
        if !self.enabled {
            return false;
        }

        self.fault_config.should_fail()
    }

    /// Get failure response details
    pub fn get_failure_response(&self) -> (u16, Option<serde_json::Value>) {
        self.fault_config.get_failure_response()
    }

    /// Process a request with latency and potential fault injection
    pub async fn process_request(
        &self,
        tags: &[String],
    ) -> Result<Option<(u16, Option<serde_json::Value>)>> {
        if !self.enabled {
            return Ok(None);
        }

        // Inject latency first
        self.inject_latency(tags).await?;

        // Check for fault injection
        if self.should_inject_failure() {
            let (status, response) = self.get_failure_response();
            return Ok(Some((status, response)));
        }

        Ok(None)
    }
}

impl Default for LatencyInjector {
    fn default() -> Self {
        Self::new(LatencyProfile::default(), FaultConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_profile_default() {
        let profile = LatencyProfile::default();
        assert_eq!(profile.base_ms, 50);
        assert_eq!(profile.jitter_ms, 20);
        assert_eq!(profile.min_ms, 0);
        assert!(profile.max_ms.is_none());
        assert!(matches!(profile.distribution, LatencyDistribution::Fixed));
    }

    #[test]
    fn test_latency_profile_new() {
        let profile = LatencyProfile::new(100, 25);
        assert_eq!(profile.base_ms, 100);
        assert_eq!(profile.jitter_ms, 25);
        assert!(matches!(profile.distribution, LatencyDistribution::Fixed));
    }

    #[test]
    fn test_latency_profile_normal_distribution() {
        let profile = LatencyProfile::with_normal_distribution(100, 20.0);
        assert_eq!(profile.base_ms, 100);
        assert!(matches!(profile.distribution, LatencyDistribution::Normal));
        assert_eq!(profile.std_dev_ms, Some(20.0));
    }

    #[test]
    fn test_latency_profile_pareto_distribution() {
        let profile = LatencyProfile::with_pareto_distribution(100, 2.5);
        assert_eq!(profile.base_ms, 100);
        assert!(matches!(profile.distribution, LatencyDistribution::Pareto));
        assert_eq!(profile.pareto_shape, Some(2.5));
    }

    #[test]
    fn test_latency_profile_with_tag_override() {
        let profile = LatencyProfile::default()
            .with_tag_override("slow".to_string(), 500)
            .with_tag_override("fast".to_string(), 10);

        assert_eq!(profile.tag_overrides.get("slow"), Some(&500));
        assert_eq!(profile.tag_overrides.get("fast"), Some(&10));
    }

    #[test]
    fn test_latency_profile_with_bounds() {
        let profile = LatencyProfile::default().with_min_ms(10).with_max_ms(1000);

        assert_eq!(profile.min_ms, 10);
        assert_eq!(profile.max_ms, Some(1000));
    }

    #[test]
    fn test_calculate_latency_with_tag_override() {
        let profile = LatencyProfile::default().with_tag_override("slow".to_string(), 500);

        let tags = vec!["slow".to_string()];
        let latency = profile.calculate_latency(&tags);
        assert_eq!(latency, Duration::from_millis(500));
    }

    #[test]
    fn test_calculate_latency_fixed_distribution() {
        let profile = LatencyProfile::new(100, 0);
        let tags = Vec::new();
        let latency = profile.calculate_latency(&tags);
        assert_eq!(latency, Duration::from_millis(100));
    }

    #[test]
    fn test_calculate_latency_respects_min_bound() {
        let profile = LatencyProfile::new(10, 0).with_min_ms(50);
        let tags = Vec::new();
        let latency = profile.calculate_latency(&tags);
        assert!(latency >= Duration::from_millis(50));
    }

    #[test]
    fn test_calculate_latency_respects_max_bound() {
        let profile = LatencyProfile::with_pareto_distribution(100, 2.0).with_max_ms(200);

        for _ in 0..100 {
            let latency = profile.calculate_latency(&[]);
            assert!(latency <= Duration::from_millis(200));
        }
    }

    #[test]
    fn test_fault_config_default() {
        let config = FaultConfig::default();
        assert_eq!(config.failure_rate, 0.0);
        assert!(!config.status_codes.is_empty());
        assert!(config.error_responses.is_empty());
    }

    #[test]
    fn test_fault_config_new() {
        let config = FaultConfig::new(0.5);
        assert_eq!(config.failure_rate, 0.5);
    }

    #[test]
    fn test_fault_config_clamps_failure_rate() {
        let config = FaultConfig::new(1.5);
        assert_eq!(config.failure_rate, 1.0);

        let config = FaultConfig::new(-0.5);
        assert_eq!(config.failure_rate, 0.0);
    }

    #[test]
    fn test_fault_config_with_status_code() {
        let config = FaultConfig::default().with_status_code(400).with_status_code(404);

        assert!(config.status_codes.contains(&400));
        assert!(config.status_codes.contains(&404));
    }

    #[test]
    fn test_fault_config_with_error_response() {
        let response = serde_json::json!({"error": "test"});
        let config =
            FaultConfig::default().with_error_response("test".to_string(), response.clone());

        assert_eq!(config.error_responses.get("test"), Some(&response));
    }

    #[test]
    fn test_fault_config_should_fail_zero_rate() {
        let config = FaultConfig::new(0.0);
        assert!(!config.should_fail());
    }

    #[test]
    fn test_fault_config_should_fail_full_rate() {
        let config = FaultConfig::new(1.0);
        assert!(config.should_fail());
    }

    #[test]
    fn test_fault_config_should_fail_probabilistic() {
        let config = FaultConfig::new(0.5);
        let mut failures = 0;
        let iterations = 1000;

        for _ in 0..iterations {
            if config.should_fail() {
                failures += 1;
            }
        }

        // Should be roughly 50% with some tolerance
        let failure_rate = failures as f64 / iterations as f64;
        assert!(failure_rate > 0.4 && failure_rate < 0.6);
    }

    #[test]
    fn test_fault_config_get_failure_response() {
        let config = FaultConfig::new(1.0).with_status_code(502);

        let (status, _) = config.get_failure_response();
        assert!(config.status_codes.contains(&status));
    }

    #[test]
    fn test_latency_injector_new() {
        let injector = LatencyInjector::new(LatencyProfile::default(), FaultConfig::default());
        assert!(injector.is_enabled());
    }

    #[test]
    fn test_latency_injector_enable_disable() {
        let mut injector = LatencyInjector::default();
        assert!(injector.is_enabled());

        injector.set_enabled(false);
        assert!(!injector.is_enabled());

        injector.set_enabled(true);
        assert!(injector.is_enabled());
    }

    #[tokio::test]
    async fn test_latency_injector_inject_latency() {
        let injector = LatencyInjector::new(LatencyProfile::new(10, 0), FaultConfig::default());

        let start = std::time::Instant::now();
        injector.inject_latency(&[]).await.unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(8));
    }

    #[tokio::test]
    async fn test_latency_injector_disabled_no_latency() {
        let mut injector =
            LatencyInjector::new(LatencyProfile::new(100, 0), FaultConfig::default());
        injector.set_enabled(false);

        let start = std::time::Instant::now();
        injector.inject_latency(&[]).await.unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed < Duration::from_millis(10));
    }

    #[test]
    fn test_latency_injector_should_inject_failure() {
        let injector = LatencyInjector::new(LatencyProfile::default(), FaultConfig::new(1.0));

        assert!(injector.should_inject_failure());
    }

    #[test]
    fn test_latency_injector_disabled_no_failure() {
        let mut injector = LatencyInjector::new(LatencyProfile::default(), FaultConfig::new(1.0));
        injector.set_enabled(false);

        assert!(!injector.should_inject_failure());
    }

    #[tokio::test]
    async fn test_latency_injector_process_request_no_failure() {
        let injector = LatencyInjector::new(LatencyProfile::new(10, 0), FaultConfig::new(0.0));

        let result = injector.process_request(&[]).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_latency_injector_process_request_with_failure() {
        let mut fault_config = FaultConfig::default();
        fault_config.failure_rate = 1.0;
        fault_config.status_codes = vec![503]; // Set to only one status code

        let injector = LatencyInjector::new(LatencyProfile::new(10, 0), fault_config);

        let result = injector.process_request(&[]).await.unwrap();
        assert!(result.is_some());

        let (status, _) = result.unwrap();
        assert_eq!(status, 503);
    }

    #[tokio::test]
    async fn test_latency_injector_process_request_disabled() {
        let mut injector = LatencyInjector::new(LatencyProfile::new(100, 0), FaultConfig::new(1.0));
        injector.set_enabled(false);

        let result = injector.process_request(&[]).await.unwrap();
        assert!(result.is_none());
    }
}
