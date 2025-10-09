//! Prometheus client for querying metrics
//!
//! Provides a client for querying Prometheus metrics API with caching support.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// Prometheus query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusResponse {
    pub status: String,
    pub data: PrometheusData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusData {
    #[serde(rename = "resultType")]
    pub result_type: String,
    pub result: Vec<PrometheusResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusResult {
    pub metric: serde_json::Value,
    pub value: Option<(f64, String)>,
    pub values: Option<Vec<(f64, String)>>,
}

/// Cached query result
#[derive(Clone)]
struct CachedResult {
    response: PrometheusResponse,
    cached_at: Instant,
}

/// Prometheus client with caching
#[derive(Clone)]
pub struct PrometheusClient {
    base_url: String,
    client: reqwest::Client,
    cache: Arc<RwLock<std::collections::HashMap<String, CachedResult>>>,
    cache_ttl: Duration,
}

impl PrometheusClient {
    /// Create a new Prometheus client
    pub fn new(prometheus_url: String) -> Self {
        Self {
            base_url: prometheus_url,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            cache_ttl: Duration::from_secs(10), // Cache for 10 seconds
        }
    }

    /// Create a client with custom cache TTL
    pub fn with_cache_ttl(prometheus_url: String, cache_ttl: Duration) -> Self {
        Self {
            base_url: prometheus_url,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            cache_ttl,
        }
    }

    /// Execute an instant query
    pub async fn query(&self, query: &str) -> Result<PrometheusResponse> {
        let cache_key = format!("instant:{}", query);

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.cached_at.elapsed() < self.cache_ttl {
                    debug!("Returning cached result for query: {}", query);
                    return Ok(cached.response.clone());
                }
            }
        }

        // Query Prometheus
        let url = format!("{}/api/v1/query", self.base_url);
        debug!("Querying Prometheus: {} with query: {}", url, query);

        let response = self
            .client
            .get(&url)
            .query(&[("query", query)])
            .send()
            .await
            .context("Failed to send request to Prometheus")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Prometheus query failed ({}): {}", status, body);
            anyhow::bail!("Prometheus query failed: {} - {}", status, body);
        }

        let result: PrometheusResponse = response
            .json()
            .await
            .context("Failed to parse Prometheus response")?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key,
                CachedResult {
                    response: result.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        Ok(result)
    }

    /// Execute a range query
    pub async fn query_range(
        &self,
        query: &str,
        start: i64,
        end: i64,
        step: &str,
    ) -> Result<PrometheusResponse> {
        let cache_key = format!("range:{}:{}:{}:{}", query, start, end, step);

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.cached_at.elapsed() < self.cache_ttl {
                    debug!("Returning cached result for range query: {}", query);
                    return Ok(cached.response.clone());
                }
            }
        }

        // Query Prometheus
        let url = format!("{}/api/v1/query_range", self.base_url);
        debug!("Querying Prometheus range: {} with query: {}", url, query);

        let response = self
            .client
            .get(&url)
            .query(&[
                ("query", query),
                ("start", &start.to_string()),
                ("end", &end.to_string()),
                ("step", step),
            ])
            .send()
            .await
            .context("Failed to send request to Prometheus")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Prometheus range query failed ({}): {}", status, body);
            anyhow::bail!("Prometheus range query failed: {} - {}", status, body);
        }

        let result: PrometheusResponse = response
            .json()
            .await
            .context("Failed to parse Prometheus response")?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key,
                CachedResult {
                    response: result.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        Ok(result)
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        debug!("Prometheus client cache cleared");
    }

    /// Extract single value from query result
    pub fn extract_single_value(response: &PrometheusResponse) -> Option<f64> {
        response
            .data
            .result
            .first()
            .and_then(|r| r.value.as_ref())
            .and_then(|(_, v)| v.parse().ok())
    }

    /// Extract multiple values from query result
    pub fn extract_values(response: &PrometheusResponse) -> Vec<(String, f64)> {
        response
            .data
            .result
            .iter()
            .filter_map(|r| {
                let label = r
                    .metric
                    .as_object()?
                    .values()
                    .next()?
                    .as_str()?
                    .to_string();
                let value: f64 = r.value.as_ref()?.1.parse().ok()?;
                Some((label, value))
            })
            .collect()
    }

    /// Extract time series data from range query
    pub fn extract_time_series(
        response: &PrometheusResponse,
    ) -> Vec<(String, Vec<(i64, f64)>)> {
        response
            .data
            .result
            .iter()
            .filter_map(|r| {
                let label = r
                    .metric
                    .as_object()?
                    .values()
                    .next()
                    .and_then(|v| v.as_str())
                    .unwrap_or("value")
                    .to_string();

                let values: Vec<(i64, f64)> = r
                    .values
                    .as_ref()?
                    .iter()
                    .filter_map(|(ts, v)| {
                        let timestamp = *ts as i64;
                        let value: f64 = v.parse().ok()?;
                        Some((timestamp, value))
                    })
                    .collect();

                Some((label, values))
            })
            .collect()
    }

    /// Check if Prometheus is reachable
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/api/v1/query", self.base_url);
        match self.client.get(&url).query(&[("query", "up")]).send().await {
            Ok(response) => response.status().is_success(),
            Err(e) => {
                warn!("Prometheus health check failed: {}", e);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = PrometheusClient::new("http://localhost:9090".to_string());
        assert_eq!(client.base_url, "http://localhost:9090");
    }

    #[test]
    fn test_client_with_custom_ttl() {
        let client = PrometheusClient::with_cache_ttl(
            "http://localhost:9090".to_string(),
            Duration::from_secs(30),
        );
        assert_eq!(client.cache_ttl, Duration::from_secs(30));
    }

    #[test]
    fn test_extract_single_value() {
        let response = PrometheusResponse {
            status: "success".to_string(),
            data: PrometheusData {
                result_type: "vector".to_string(),
                result: vec![PrometheusResult {
                    metric: serde_json::json!({}),
                    value: Some((1234567890.0, "125.5".to_string())),
                    values: None,
                }],
            },
        };

        let value = PrometheusClient::extract_single_value(&response);
        assert_eq!(value, Some(125.5));
    }
}
