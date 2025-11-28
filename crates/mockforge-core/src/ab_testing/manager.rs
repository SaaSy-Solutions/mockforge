//! Variant manager for A/B testing
//!
//! This module provides functionality for managing mock variants and A/B test configurations.

use crate::ab_testing::types::{ABTestConfig, MockVariant, VariantAnalytics};
use crate::error::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Manages A/B test configurations and variants
#[derive(Debug, Clone)]
pub struct VariantManager {
    /// A/B test configurations indexed by endpoint key: "{method} {path}"
    tests: Arc<RwLock<HashMap<String, ABTestConfig>>>,
    /// Analytics data for variants indexed by endpoint key and variant ID
    analytics: Arc<RwLock<HashMap<String, HashMap<String, VariantAnalytics>>>>,
    /// Round-robin counters for round-robin strategy
    round_robin_counters: Arc<RwLock<HashMap<String, usize>>>,
}

impl VariantManager {
    /// Create a new variant manager
    pub fn new() -> Self {
        Self {
            tests: Arc::new(RwLock::new(HashMap::new())),
            analytics: Arc::new(RwLock::new(HashMap::new())),
            round_robin_counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an A/B test configuration
    pub async fn register_test(&self, config: ABTestConfig) -> Result<()> {
        // Validate allocations
        config.validate_allocations().map_err(Error::validation)?;

        let key = Self::endpoint_key(&config.method, &config.endpoint_path);
        let mut tests = self.tests.write().await;
        tests.insert(key.clone(), config.clone());

        // Initialize analytics for all variants
        let mut analytics = self.analytics.write().await;
        let variant_analytics = analytics.entry(key).or_insert_with(HashMap::new);
        for variant in &config.variants {
            variant_analytics.insert(
                variant.variant_id.clone(),
                VariantAnalytics::new(variant.variant_id.clone()),
            );
        }

        info!(
            "Registered A/B test '{}' for {} {} with {} variants",
            config.test_name,
            config.method,
            config.endpoint_path,
            config.variants.len()
        );

        Ok(())
    }

    /// Get A/B test configuration for an endpoint
    pub async fn get_test(&self, method: &str, path: &str) -> Option<ABTestConfig> {
        let key = Self::endpoint_key(method, path);
        let tests = self.tests.read().await;
        tests.get(&key).cloned()
    }

    /// List all registered A/B tests
    pub async fn list_tests(&self) -> Vec<ABTestConfig> {
        let tests = self.tests.read().await;
        tests.values().cloned().collect()
    }

    /// Remove an A/B test configuration
    pub async fn remove_test(&self, method: &str, path: &str) -> Result<()> {
        let key = Self::endpoint_key(method, path);
        let mut tests = self.tests.write().await;
        tests.remove(&key);

        // Optionally clear analytics (or keep for historical data)
        // For now, we'll keep analytics even after test removal

        info!("Removed A/B test for {} {}", method, path);
        Ok(())
    }

    /// Get a variant by ID for an endpoint
    pub async fn get_variant(
        &self,
        method: &str,
        path: &str,
        variant_id: &str,
    ) -> Option<MockVariant> {
        if let Some(config) = self.get_test(method, path).await {
            config.variants.iter().find(|v| v.variant_id == variant_id).cloned()
        } else {
            None
        }
    }

    /// Record analytics for a variant request
    pub async fn record_request(
        &self,
        method: &str,
        path: &str,
        variant_id: &str,
        status_code: u16,
        response_time_ms: f64,
    ) {
        let key = Self::endpoint_key(method, path);
        let mut analytics = self.analytics.write().await;
        if let Some(variant_analytics) = analytics.get_mut(&key) {
            if let Some(analytics_data) = variant_analytics.get_mut(variant_id) {
                analytics_data.record_request(status_code, response_time_ms);
            } else {
                // Initialize analytics if not present
                let mut new_analytics = VariantAnalytics::new(variant_id.to_string());
                new_analytics.record_request(status_code, response_time_ms);
                variant_analytics.insert(variant_id.to_string(), new_analytics);
            }
        }
    }

    /// Get analytics for a variant
    pub async fn get_variant_analytics(
        &self,
        method: &str,
        path: &str,
        variant_id: &str,
    ) -> Option<VariantAnalytics> {
        let key = Self::endpoint_key(method, path);
        let analytics = self.analytics.read().await;
        analytics.get(&key)?.get(variant_id).cloned()
    }

    /// Get all analytics for an endpoint
    pub async fn get_endpoint_analytics(
        &self,
        method: &str,
        path: &str,
    ) -> HashMap<String, VariantAnalytics> {
        let key = Self::endpoint_key(method, path);
        let analytics = self.analytics.read().await;
        analytics.get(&key).cloned().unwrap_or_default()
    }

    /// Get round-robin counter for an endpoint
    pub async fn get_round_robin_index(&self, method: &str, path: &str) -> usize {
        let key = Self::endpoint_key(method, path);
        let mut counters = self.round_robin_counters.write().await;
        let counter = counters.entry(key).or_insert(0);
        *counter
    }

    /// Increment round-robin counter for an endpoint
    pub async fn increment_round_robin(&self, method: &str, path: &str, max: usize) -> usize {
        let key = Self::endpoint_key(method, path);
        let mut counters = self.round_robin_counters.write().await;
        let counter = counters.entry(key).or_insert(0);
        let current = *counter;
        *counter = (*counter + 1) % max;
        current
    }

    /// Generate a consistent hash for a request attribute
    ///
    /// This is used for consistent hashing strategy to ensure the same
    /// request attribute (e.g., user ID) always gets the same variant.
    pub fn consistent_hash(attribute: &str, num_variants: usize) -> usize {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        attribute.hash(&mut hasher);
        (hasher.finish() as usize) % num_variants
    }

    /// Generate endpoint key from method and path
    fn endpoint_key(method: &str, path: &str) -> String {
        format!("{} {}", method.to_uppercase(), path)
    }
}

impl Default for VariantManager {
    fn default() -> Self {
        Self::new()
    }
}
