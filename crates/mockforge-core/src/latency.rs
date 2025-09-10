//! Latency simulation and fault injection for MockForge

use crate::Result;
use rand::Rng;
use std::collections::HashMap;
use std::time::Duration;

/// Latency profile configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LatencyProfile {
    /// Base latency in milliseconds
    pub base_ms: u64,
    /// Random jitter range in milliseconds (added to base)
    pub jitter_ms: u64,
    /// Tag-based latency overrides
    pub tag_overrides: HashMap<String, u64>,
}

impl Default for LatencyProfile {
    fn default() -> Self {
        Self {
            base_ms: 50,   // 50ms base latency
            jitter_ms: 20, // ±20ms jitter
            tag_overrides: HashMap::new(),
        }
    }
}

impl LatencyProfile {
    /// Create a new latency profile
    pub fn new(base_ms: u64, jitter_ms: u64) -> Self {
        Self {
            base_ms,
            jitter_ms,
            tag_overrides: HashMap::new(),
        }
    }

    /// Add a tag-based latency override
    pub fn with_tag_override(mut self, tag: String, latency_ms: u64) -> Self {
        self.tag_overrides.insert(tag, latency_ms);
        self
    }

    /// Calculate latency for a request with optional tags
    pub fn calculate_latency(&self, tags: &[String]) -> Duration {
        let mut rng = rand::rng();

        // Check for tag overrides (use the first matching tag)
        let base_ms = tags
            .iter()
            .find_map(|tag| self.tag_overrides.get(tag))
            .copied()
            .unwrap_or(self.base_ms);

        // Add random jitter
        let jitter = if self.jitter_ms > 0 {
            rng.random_range(0..=self.jitter_ms * 2).saturating_sub(self.jitter_ms)
        } else {
            0
        };

        let total_ms = base_ms.saturating_add(jitter);
        Duration::from_millis(total_ms)
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
#[derive(Debug)]
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
