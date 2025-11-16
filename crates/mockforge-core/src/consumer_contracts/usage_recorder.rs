//! Usage recorder for tracking consumer field usage
//!
//! This module provides functionality for extracting and recording which fields
//! consumers actually use from API responses.

use crate::consumer_contracts::types::{ConsumerIdentifier, ConsumerUsage};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Recorder for tracking consumer usage
#[derive(Debug, Clone)]
pub struct UsageRecorder {
    /// Usage data indexed by consumer ID and endpoint
    usage: Arc<RwLock<HashMap<String, HashMap<String, ConsumerUsage>>>>,
}

impl UsageRecorder {
    /// Create a new usage recorder
    pub fn new() -> Self {
        Self {
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record usage from a request/response
    ///
    /// Extracts field paths from the response body and records them for the consumer.
    pub async fn record_usage(
        &self,
        consumer_id: &str,
        endpoint: &str,
        method: &str,
        response_body: Option<&Value>,
    ) {
        // Extract field paths from response body
        let fields_used = if let Some(body) = response_body {
            Self::extract_field_paths(body, "")
        } else {
            vec![]
        };

        // Get or create usage entry
        let key = format!("{} {}", method, endpoint);
        let mut usage_map = self.usage.write().await;

        let usage = usage_map
            .entry(consumer_id.to_string())
            .or_insert_with(HashMap::new)
            .entry(key.clone())
            .or_insert_with(|| ConsumerUsage {
                consumer_id: consumer_id.to_string(),
                endpoint: endpoint.to_string(),
                method: method.to_string(),
                fields_used: vec![],
                last_used_at: chrono::Utc::now().timestamp(),
                usage_count: 0,
            });

        // Update usage
        usage.last_used_at = chrono::Utc::now().timestamp();
        usage.usage_count += 1;

        // Merge new fields with existing
        for field in fields_used {
            if !usage.fields_used.contains(&field) {
                usage.fields_used.push(field);
            }
        }
    }

    /// Get usage for a consumer
    pub async fn get_usage(&self, consumer_id: &str) -> Vec<ConsumerUsage> {
        let usage_map = self.usage.read().await;
        usage_map
            .get(consumer_id)
            .map(|endpoint_map| endpoint_map.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get usage for a specific endpoint
    pub async fn get_endpoint_usage(
        &self,
        consumer_id: &str,
        endpoint: &str,
        method: &str,
    ) -> Option<ConsumerUsage> {
        let key = format!("{} {}", method, endpoint);
        let usage_map = self.usage.read().await;
        usage_map
            .get(consumer_id)
            .and_then(|endpoint_map| endpoint_map.get(&key))
            .cloned()
    }

    /// Extract field paths from a JSON value
    fn extract_field_paths(value: &Value, prefix: &str) -> Vec<String> {
        let mut paths = Vec::new();

        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    paths.push(path.clone());
                    paths.extend(Self::extract_field_paths(val, &path));
                }
            }
            Value::Array(arr) => {
                if !arr.is_empty() {
                    // For arrays, extract paths from first element
                    paths.extend(Self::extract_field_paths(&arr[0], prefix));
                }
            }
            _ => {
                // Primitive value - path is already added
            }
        }

        paths
    }
}

impl Default for UsageRecorder {
    fn default() -> Self {
        Self::new()
    }
}
