//! Traffic shaping beyond latency simulation
//!
//! This module provides advanced traffic shaping capabilities including:
//! - Bandwidth throttling using token bucket algorithm
//! - Burst packet loss simulation
//! - Integration with existing latency and fault injection

use crate::{Error, Result};
use rand::Rng;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

const GLOBAL_BUCKET_KEY: &str = "__global__";

/// Bandwidth throttling configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BandwidthConfig {
    /// Enable bandwidth throttling
    pub enabled: bool,
    /// Maximum bandwidth in bytes per second (0 = unlimited)
    pub max_bytes_per_sec: u64,
    /// Token bucket capacity in bytes (burst allowance)
    pub burst_capacity_bytes: u64,
    /// Tag-based bandwidth overrides
    pub tag_overrides: HashMap<String, u64>,
}

impl Default for BandwidthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_bytes_per_sec: 0,              // Unlimited
            burst_capacity_bytes: 1024 * 1024, // 1MB burst capacity
            tag_overrides: HashMap::new(),
        }
    }
}

impl BandwidthConfig {
    /// Create a new bandwidth configuration
    pub fn new(max_bytes_per_sec: u64, burst_capacity_bytes: u64) -> Self {
        Self {
            enabled: true,
            max_bytes_per_sec,
            burst_capacity_bytes,
            tag_overrides: HashMap::new(),
        }
    }

    /// Add a tag-based bandwidth override
    pub fn with_tag_override(mut self, tag: String, max_bytes_per_sec: u64) -> Self {
        self.tag_overrides.insert(tag, max_bytes_per_sec);
        self
    }

    /// Get the effective bandwidth limit for the given tags
    pub fn get_effective_limit(&self, tags: &[String]) -> u64 {
        // Check for tag overrides (use the first matching tag)
        if let Some(&override_limit) = tags.iter().find_map(|tag| self.tag_overrides.get(tag)) {
            return override_limit;
        }
        self.max_bytes_per_sec
    }
}

/// Burst loss configuration for simulating intermittent connectivity issues
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BurstLossConfig {
    /// Enable burst loss simulation
    pub enabled: bool,
    /// Probability of entering a loss burst (0.0 to 1.0)
    pub burst_probability: f64,
    /// Duration of loss burst in milliseconds
    pub burst_duration_ms: u64,
    /// Packet loss rate during burst (0.0 to 1.0)
    pub loss_rate_during_burst: f64,
    /// Recovery time between bursts in milliseconds
    pub recovery_time_ms: u64,
    /// Tag-based burst loss overrides
    pub tag_overrides: HashMap<String, BurstLossOverride>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BurstLossOverride {
    pub burst_probability: f64,
    pub burst_duration_ms: u64,
    pub loss_rate_during_burst: f64,
    pub recovery_time_ms: u64,
}

impl Default for BurstLossConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            burst_probability: 0.1,      // 10% chance of burst
            burst_duration_ms: 5000,     // 5 second bursts
            loss_rate_during_burst: 0.5, // 50% loss during burst
            recovery_time_ms: 30000,     // 30 second recovery
            tag_overrides: HashMap::new(),
        }
    }
}

impl BurstLossConfig {
    /// Create a new burst loss configuration
    pub fn new(
        burst_probability: f64,
        burst_duration_ms: u64,
        loss_rate: f64,
        recovery_time_ms: u64,
    ) -> Self {
        Self {
            enabled: true,
            burst_probability: burst_probability.clamp(0.0, 1.0),
            burst_duration_ms,
            loss_rate_during_burst: loss_rate.clamp(0.0, 1.0),
            recovery_time_ms,
            tag_overrides: HashMap::new(),
        }
    }

    /// Add a tag-based burst loss override
    pub fn with_tag_override(mut self, tag: String, override_config: BurstLossOverride) -> Self {
        self.tag_overrides.insert(tag, override_config);
        self
    }

    /// Get the effective burst loss config for the given tags
    pub fn effective_config<'a>(&'a self, tags: &[String]) -> Cow<'a, BurstLossConfig> {
        if let Some(override_config) = tags.iter().find_map(|tag| self.tag_overrides.get(tag)) {
            let mut temp_config = self.clone();
            temp_config.burst_probability = override_config.burst_probability;
            temp_config.burst_duration_ms = override_config.burst_duration_ms;
            temp_config.loss_rate_during_burst = override_config.loss_rate_during_burst;
            temp_config.recovery_time_ms = override_config.recovery_time_ms;
            Cow::Owned(temp_config)
        } else {
            Cow::Borrowed(self)
        }
    }
}

/// Token bucket for bandwidth throttling
#[derive(Debug)]
struct TokenBucket {
    /// Current number of tokens (bytes that can be sent)
    tokens: f64,
    /// Maximum capacity of the bucket
    capacity: f64,
    /// Rate of token replenishment (tokens per second)
    refill_rate: f64,
    /// Last refill timestamp
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket
    fn new(capacity: u64, refill_rate_bytes_per_sec: u64) -> Self {
        Self {
            tokens: capacity as f64,
            capacity: capacity as f64,
            refill_rate: refill_rate_bytes_per_sec as f64,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let tokens_to_add = elapsed * self.refill_rate;

        self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
        self.last_refill = now;
    }

    /// Try to consume tokens for the given number of bytes
    fn try_consume(&mut self, bytes: u64) -> bool {
        self.refill();
        if self.tokens >= bytes as f64 {
            self.tokens -= bytes as f64;
            true
        } else {
            false
        }
    }

    /// Get the time to wait until enough tokens are available
    fn time_until_available(&mut self, bytes: u64) -> Duration {
        self.refill();
        if self.tokens >= bytes as f64 {
            Duration::ZERO
        } else {
            let tokens_needed = bytes as f64 - self.tokens;
            let seconds_needed = tokens_needed / self.refill_rate;
            Duration::from_secs_f64(seconds_needed)
        }
    }
}

/// Burst loss state machine
#[derive(Debug)]
struct BurstLossState {
    /// Whether currently in a loss burst
    in_burst: bool,
    /// When the current burst started
    burst_start: Option<Instant>,
    /// When the current recovery period started
    recovery_start: Option<Instant>,
}

impl BurstLossState {
    fn new() -> Self {
        Self {
            in_burst: false,
            burst_start: None,
            recovery_start: None,
        }
    }

    /// Determine if a packet should be lost based on current state
    fn should_drop_packet(&mut self, config: &BurstLossConfig) -> bool {
        if !config.enabled {
            return false;
        }

        let now = Instant::now();

        match (self.in_burst, self.burst_start, self.recovery_start) {
            (true, Some(burst_start), _) => {
                // Currently in burst - check if burst should end
                let burst_duration = now.duration_since(burst_start);
                if burst_duration >= Duration::from_millis(config.burst_duration_ms) {
                    // End burst and start recovery
                    self.in_burst = false;
                    self.burst_start = None;
                    self.recovery_start = Some(now);
                    false // Don't drop this packet
                } else {
                    // Still in burst - apply loss rate
                    let mut rng = rand::rng();
                    rng.random_bool(config.loss_rate_during_burst)
                }
            }
            (true, None, _) => {
                // Invalid state: in burst but no burst start time - reset to normal
                self.in_burst = false;
                false
            }
            (false, _, Some(recovery_start)) => {
                // In recovery - check if recovery should end
                let recovery_duration = now.duration_since(recovery_start);
                if recovery_duration >= Duration::from_millis(config.recovery_time_ms) {
                    // End recovery
                    self.recovery_start = None;
                    // Check if we should start a new burst
                    let mut rng = rand::rng();
                    if rng.random_bool(config.burst_probability) {
                        self.in_burst = true;
                        self.burst_start = Some(now);
                        // Apply loss rate for the first packet of the burst
                        rng.random_bool(config.loss_rate_during_burst)
                    } else {
                        false
                    }
                } else {
                    false // Still in recovery
                }
            }
            (false, _, None) => {
                // Not in burst or recovery - check if we should start a burst
                let mut rng = rand::rng();
                if rng.random_bool(config.burst_probability) {
                    self.in_burst = true;
                    self.burst_start = Some(now);
                    rng.random_bool(config.loss_rate_during_burst)
                } else {
                    false
                }
            }
        }
    }
}

/// Traffic shaping configuration combining all features
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct TrafficShapingConfig {
    /// Bandwidth throttling configuration
    pub bandwidth: BandwidthConfig,
    /// Burst loss configuration
    pub burst_loss: BurstLossConfig,
}

/// Main traffic shaper combining bandwidth throttling and burst loss
#[derive(Debug, Clone)]
pub struct TrafficShaper {
    /// Bandwidth configuration
    bandwidth_config: BandwidthConfig,
    /// Burst loss configuration
    burst_loss_config: BurstLossConfig,
    /// Token buckets keyed by effective tag/group
    token_buckets: Arc<RwLock<HashMap<String, Arc<Mutex<TokenBucket>>>>>,
    /// Burst loss state
    burst_loss_state: Arc<Mutex<BurstLossState>>,
}

impl TrafficShaper {
    /// Create a new traffic shaper
    pub fn new(config: TrafficShapingConfig) -> Self {
        Self {
            bandwidth_config: config.bandwidth,
            burst_loss_config: config.burst_loss,
            token_buckets: Arc::new(RwLock::new(HashMap::new())),
            burst_loss_state: Arc::new(Mutex::new(BurstLossState::new())),
        }
    }

    /// Apply bandwidth throttling to a data transfer
    pub async fn throttle_bandwidth(&self, data_size: u64, tags: &[String]) -> Result<()> {
        if !self.bandwidth_config.enabled {
            return Ok(());
        }

        let (bucket_key, effective_limit) = self.resolve_bandwidth_bucket(tags);

        if effective_limit == 0 {
            return Ok(());
        }

        let bucket_arc = self.get_or_create_bucket(&bucket_key, effective_limit).await;

        {
            let mut bucket = bucket_arc.lock().await;
            if bucket.try_consume(data_size) {
                return Ok(());
            }

            let wait_time = bucket.time_until_available(data_size);
            drop(bucket);

            if wait_time.is_zero() {
                return Err(Error::generic(format!(
                    "Failed to acquire bandwidth tokens for {} bytes",
                    data_size
                )));
            }

            tokio::time::sleep(wait_time).await;
        }

        let mut bucket = bucket_arc.lock().await;
        if bucket.try_consume(data_size) {
            Ok(())
        } else {
            Err(Error::generic(format!(
                "Failed to acquire bandwidth tokens for {} bytes",
                data_size
            )))
        }
    }

    /// Check if a packet should be dropped due to burst loss
    pub async fn should_drop_packet(&self, tags: &[String]) -> bool {
        if !self.burst_loss_config.enabled {
            return false;
        }

        let effective_config = self.burst_loss_config.effective_config(tags);
        let mut state = self.burst_loss_state.lock().await;
        state.should_drop_packet(effective_config.as_ref())
    }

    /// Process a data transfer with both bandwidth throttling and burst loss
    pub async fn process_transfer(
        &self,
        data_size: u64,
        tags: &[String],
    ) -> Result<Option<Duration>> {
        // First, apply bandwidth throttling
        self.throttle_bandwidth(data_size, tags).await?;

        // Then, check for burst loss
        if self.should_drop_packet(tags).await {
            return Ok(Some(Duration::from_millis(100))); // Simulate packet timeout
        }

        Ok(None)
    }

    /// Get current bandwidth usage statistics
    pub async fn get_bandwidth_stats(&self) -> BandwidthStats {
        let maybe_bucket = {
            let guard = self.token_buckets.read().await;
            guard.get(GLOBAL_BUCKET_KEY).cloned()
        };

        if let Some(bucket_arc) = maybe_bucket {
            let bucket = bucket_arc.lock().await;
            BandwidthStats {
                current_tokens: bucket.tokens as u64,
                capacity: bucket.capacity as u64,
                refill_rate_bytes_per_sec: bucket.refill_rate as u64,
            }
        } else {
            BandwidthStats {
                current_tokens: self.bandwidth_config.burst_capacity_bytes,
                capacity: self.bandwidth_config.burst_capacity_bytes,
                refill_rate_bytes_per_sec: self.bandwidth_config.max_bytes_per_sec,
            }
        }
    }

    /// Get current burst loss state
    pub async fn get_burst_loss_stats(&self) -> BurstLossStats {
        let state = self.burst_loss_state.lock().await;
        BurstLossStats {
            in_burst: state.in_burst,
            burst_start: state.burst_start,
            recovery_start: state.recovery_start,
        }
    }

    async fn get_or_create_bucket(
        &self,
        bucket_key: &str,
        effective_limit: u64,
    ) -> Arc<Mutex<TokenBucket>> {
        if let Some(existing) = self.token_buckets.read().await.get(bucket_key).cloned() {
            return existing;
        }

        let mut buckets = self.token_buckets.write().await;
        buckets
            .entry(bucket_key.to_string())
            .or_insert_with(|| {
                Arc::new(Mutex::new(TokenBucket::new(
                    self.bandwidth_config.burst_capacity_bytes,
                    effective_limit,
                )))
            })
            .clone()
    }

    fn resolve_bandwidth_bucket(&self, tags: &[String]) -> (String, u64) {
        if let Some((tag, limit)) = tags.iter().find_map(|tag| {
            self.bandwidth_config.tag_overrides.get(tag).map(|limit| (tag.as_str(), *limit))
        }) {
            (format!("tag:{}", tag), limit)
        } else {
            (GLOBAL_BUCKET_KEY.to_string(), self.bandwidth_config.max_bytes_per_sec)
        }
    }
}

/// Bandwidth usage statistics
#[derive(Debug, Clone)]
pub struct BandwidthStats {
    pub current_tokens: u64,
    pub capacity: u64,
    pub refill_rate_bytes_per_sec: u64,
}

/// Burst loss state statistics
#[derive(Debug, Clone)]
pub struct BurstLossStats {
    pub in_burst: bool,
    pub burst_start: Option<Instant>,
    pub recovery_start: Option<Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_bandwidth_throttling() {
        let config = TrafficShapingConfig {
            bandwidth: BandwidthConfig::new(1000, 100), // 1000 bytes/sec, 100 byte burst
            burst_loss: BurstLossConfig::default(),
        };
        let shaper = TrafficShaper::new(config);

        // Small transfer should succeed immediately
        let result = shaper.throttle_bandwidth(50, &[]).await;
        assert!(result.is_ok());

        // Large transfer should be throttled (but within burst capacity)
        let start = Instant::now();
        let result = shaper.throttle_bandwidth(80, &[]).await; // 50 + 80 = 130 total, need to wait for refill
        let elapsed = start.elapsed();
        assert!(result.is_ok());
        // Should have waited at least some time due to throttling
        assert!(elapsed >= Duration::from_millis(30)); // At least 30ms for 80 additional bytes at 1000 bytes/sec
    }

    #[tokio::test]
    async fn test_burst_loss() {
        let config = TrafficShapingConfig {
            bandwidth: BandwidthConfig::default(),
            burst_loss: BurstLossConfig::new(1.0, 1000, 1.0, 1000), // 100% burst probability, 100% loss
        };
        let shaper = TrafficShaper::new(config);

        // First packet should trigger burst and be dropped
        let should_drop = shaper.should_drop_packet(&[]).await;
        assert!(should_drop);

        // Subsequent packets in burst should also be dropped
        for _ in 0..5 {
            let should_drop = shaper.should_drop_packet(&[]).await;
            assert!(should_drop);
        }
    }

    #[tokio::test]
    async fn test_bandwidth_tag_override_with_global_unlimited() {
        let mut bandwidth = BandwidthConfig::default();
        bandwidth.enabled = true;
        bandwidth.max_bytes_per_sec = 0;
        bandwidth.burst_capacity_bytes = 100;
        bandwidth = bandwidth.with_tag_override("limited".to_string(), 100);

        let shaper = TrafficShaper::new(TrafficShapingConfig {
            bandwidth,
            burst_loss: BurstLossConfig::default(),
        });

        let tags = vec!["limited".to_string()];
        shaper
            .throttle_bandwidth(100, &tags)
            .await
            .expect("initial transfer should succeed immediately");

        let start = Instant::now();
        shaper
            .throttle_bandwidth(100, &tags)
            .await
            .expect("tag override should throttle but eventually succeed");
        assert!(
            start.elapsed() >= Duration::from_millis(900),
            "override-specific transfer should respect configured rate"
        );
    }

    #[test]
    fn test_bandwidth_config_overrides() {
        let mut config = BandwidthConfig::new(1000, 100);
        config = config.with_tag_override("high-priority".to_string(), 5000);

        assert_eq!(config.get_effective_limit(&[]), 1000);
        assert_eq!(config.get_effective_limit(&["high-priority".to_string()]), 5000);
        assert_eq!(
            config.get_effective_limit(&["low-priority".to_string(), "high-priority".to_string()]),
            5000
        );
    }

    #[test]
    fn test_burst_loss_effective_config_override() {
        let override_cfg = BurstLossOverride {
            burst_probability: 0.8,
            burst_duration_ms: 2000,
            loss_rate_during_burst: 0.9,
            recovery_time_ms: 5000,
        };

        let config =
            BurstLossConfig::default().with_tag_override("flaky".to_string(), override_cfg.clone());

        let effective = config.effective_config(&["flaky".to_string()]);
        assert_eq!(effective.burst_probability, override_cfg.burst_probability);
        assert_eq!(effective.burst_duration_ms, override_cfg.burst_duration_ms);
        assert_eq!(effective.loss_rate_during_burst, override_cfg.loss_rate_during_burst);
        assert_eq!(effective.recovery_time_ms, override_cfg.recovery_time_ms);
    }
}
