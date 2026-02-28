//! Types for A/B testing mock variants
//!
//! This module defines the core data structures for managing multiple
//! mock variants per endpoint and routing traffic between them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A mock variant for an endpoint
///
/// Each variant represents a different response configuration for the same endpoint.
/// For example, `GET /users/{id}` might have variants:
/// - `new_user`: Returns a welcome message for new users
/// - `existing_user`: Returns standard user data
/// - `premium_user`: Returns enhanced user data with premium features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockVariant {
    /// Unique identifier for this variant (e.g., "new_user", "existing_user")
    pub variant_id: String,
    /// Human-readable name for this variant
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// HTTP status code for this variant
    pub status_code: u16,
    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body (JSON, text, or binary)
    pub body: Value,
    /// Optional latency simulation in milliseconds
    pub latency_ms: Option<u64>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl MockVariant {
    /// Create a new mock variant
    pub fn new(variant_id: String, name: String, status_code: u16, body: Value) -> Self {
        Self {
            variant_id,
            name,
            description: None,
            status_code,
            headers: HashMap::new(),
            body,
            latency_ms: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a response header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Set latency simulation
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

/// Allocation configuration for a variant
///
/// Defines what percentage of traffic should be routed to this variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantAllocation {
    /// Variant ID
    pub variant_id: String,
    /// Percentage of traffic (0.0-100.0)
    pub percentage: f64,
    /// Optional conditions for this allocation (e.g., user agent, IP range)
    #[serde(default)]
    pub conditions: HashMap<String, Value>,
}

impl VariantAllocation {
    /// Create a new variant allocation
    pub fn new(variant_id: String, percentage: f64) -> Self {
        Self {
            variant_id,
            percentage,
            conditions: HashMap::new(),
        }
    }

    /// Add a condition
    pub fn with_condition(mut self, key: String, value: Value) -> Self {
        self.conditions.insert(key, value);
        self
    }
}

/// Strategy for selecting variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum VariantSelectionStrategy {
    /// Random selection based on allocation percentages
    #[default]
    Random,
    /// Consistent hashing based on request attributes (e.g., user ID, IP)
    ConsistentHash,
    /// Round-robin selection
    RoundRobin,
    /// Sticky session (same variant for same session)
    StickySession,
}

/// A/B test configuration for an endpoint
///
/// Defines multiple variants and how traffic should be distributed among them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestConfig {
    /// Endpoint path pattern (e.g., "/api/users/{id}")
    pub endpoint_path: String,
    /// HTTP method (e.g., "GET", "POST")
    pub method: String,
    /// Test name/identifier
    pub test_name: String,
    /// Test description
    pub description: Option<String>,
    /// List of variants for this endpoint
    pub variants: Vec<MockVariant>,
    /// Allocation configuration (how traffic is distributed)
    pub allocations: Vec<VariantAllocation>,
    /// Selection strategy
    #[serde(default)]
    pub strategy: VariantSelectionStrategy,
    /// Whether this A/B test is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional start time (test only runs after this time)
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Optional end time (test stops after this time)
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Tags for organization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

fn default_true() -> bool {
    true
}

impl ABTestConfig {
    /// Create a new A/B test configuration
    pub fn new(
        endpoint_path: String,
        method: String,
        test_name: String,
        variants: Vec<MockVariant>,
        allocations: Vec<VariantAllocation>,
    ) -> Self {
        Self {
            endpoint_path,
            method,
            test_name,
            description: None,
            variants,
            allocations,
            strategy: VariantSelectionStrategy::default(),
            enabled: true,
            start_time: None,
            end_time: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set selection strategy
    pub fn with_strategy(mut self, strategy: VariantSelectionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Enable or disable the test
    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set time window
    pub fn with_time_window(
        mut self,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.start_time = start_time;
        self.end_time = end_time;
        self
    }

    /// Validate that allocations sum to 100%
    pub fn validate_allocations(&self) -> Result<(), String> {
        let total: f64 = self.allocations.iter().map(|a| a.percentage).sum();
        if (total - 100.0).abs() > 0.01 {
            return Err(format!("Allocations must sum to 100%, got {}%", total));
        }

        // Validate that all variant IDs in allocations exist in variants
        let variant_ids: std::collections::HashSet<&str> =
            self.variants.iter().map(|v| v.variant_id.as_str()).collect();
        for allocation in &self.allocations {
            if !variant_ids.contains(allocation.variant_id.as_str()) {
                return Err(format!(
                    "Allocation references unknown variant: {}",
                    allocation.variant_id
                ));
            }
        }

        Ok(())
    }
}

/// Analytics data for a variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantAnalytics {
    /// Variant ID
    pub variant_id: String,
    /// Number of requests served
    pub request_count: u64,
    /// Number of successful responses (2xx)
    pub success_count: u64,
    /// Number of error responses (4xx, 5xx)
    pub error_count: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Total response time in milliseconds (for calculating average)
    pub total_response_time_ms: f64,
    /// Timestamp of first request
    pub first_request_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp of last request
    pub last_request_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl VariantAnalytics {
    /// Create new analytics for a variant
    pub fn new(variant_id: String) -> Self {
        Self {
            variant_id,
            request_count: 0,
            success_count: 0,
            error_count: 0,
            avg_response_time_ms: 0.0,
            total_response_time_ms: 0.0,
            first_request_at: None,
            last_request_at: None,
        }
    }

    /// Record a request
    pub fn record_request(&mut self, status_code: u16, response_time_ms: f64) {
        self.request_count += 1;
        self.total_response_time_ms += response_time_ms;
        self.avg_response_time_ms = self.total_response_time_ms / self.request_count as f64;

        if (200..300).contains(&status_code) {
            self.success_count += 1;
        } else if (400..600).contains(&status_code) {
            self.error_count += 1;
        }

        let now = chrono::Utc::now();
        if self.first_request_at.is_none() {
            self.first_request_at = Some(now);
        }
        self.last_request_at = Some(now);
    }

    /// Get success rate (0.0-1.0)
    pub fn success_rate(&self) -> f64 {
        if self.request_count == 0 {
            return 0.0;
        }
        self.success_count as f64 / self.request_count as f64
    }

    /// Get error rate (0.0-1.0)
    pub fn error_rate(&self) -> f64 {
        if self.request_count == 0 {
            return 0.0;
        }
        self.error_count as f64 / self.request_count as f64
    }
}
