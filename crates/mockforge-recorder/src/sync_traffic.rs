//! Traffic-aware sync filtering
//!
//! This module provides functionality to filter and prioritize endpoints for sync
//! based on usage statistics and Reality Continuum blend ratios.

use crate::sync::{DetectedChange, TrafficAwareConfig};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// Endpoint usage statistics aggregated across all consumers
#[derive(Debug, Clone)]
pub struct EndpointUsageStats {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Total request count across all consumers
    pub total_requests: u64,
    /// Last usage timestamp
    pub last_used_at: Option<DateTime<Utc>>,
    /// Number of unique consumers
    pub unique_consumers: usize,
}

/// Priority score for an endpoint
#[derive(Debug, Clone)]
pub struct EndpointPriority {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Priority score (higher = more important)
    pub score: f64,
    /// Request count
    pub request_count: u64,
    /// Recency score (0.0 to 1.0, 1.0 = most recent)
    pub recency_score: f64,
    /// Reality ratio (0.0 = all mock, 1.0 = all real)
    pub reality_ratio: f64,
}

/// Traffic analyzer for sync operations
pub struct TrafficAnalyzer {
    config: TrafficAwareConfig,
}

impl TrafficAnalyzer {
    /// Create a new traffic analyzer
    pub fn new(config: TrafficAwareConfig) -> Self {
        Self { config }
    }

    /// Aggregate usage statistics from database requests
    ///
    /// This aggregates usage from recorded requests in the database
    pub async fn aggregate_usage_stats_from_db(
        &self,
        database: &crate::database::RecorderDatabase,
    ) -> HashMap<String, EndpointUsageStats> {
        let mut aggregated: HashMap<String, EndpointUsageStats> = HashMap::new();
        let cutoff_time = Utc::now() - Duration::days(self.config.lookback_days as i64);

        // Get recent requests from database
        if let Ok(requests) = database.list_recent(10000).await {
            for request in requests {
                let key = format!("{} {}", request.method, request.path);

                // Use timestamp directly (it's already DateTime<Utc>)
                let request_time = if request.timestamp >= cutoff_time {
                    Some(request.timestamp)
                } else {
                    None
                };

                let stats = aggregated.entry(key.clone()).or_insert_with(|| EndpointUsageStats {
                    endpoint: request.path.clone(),
                    method: request.method.clone(),
                    total_requests: 0,
                    last_used_at: None,
                    unique_consumers: 0,
                });

                stats.total_requests += 1;

                // Update last used time if this is more recent
                if let Some(rt) = request_time {
                    if stats.last_used_at.is_none_or(|last| rt > last) {
                        stats.last_used_at = Some(rt);
                    }
                }
            }
        }

        aggregated
    }

    /// Calculate priority scores for endpoints
    pub fn calculate_priorities(
        &self,
        usage_stats: &HashMap<String, EndpointUsageStats>,
        reality_ratios: &HashMap<String, f64>,
    ) -> Vec<EndpointPriority> {
        let mut priorities = Vec::new();
        let now = Utc::now();
        let lookback_duration = Duration::days(self.config.lookback_days as i64);
        let cutoff_time = now - lookback_duration;

        for stats in usage_stats.values() {
            // Skip if last used before lookback window
            if let Some(last_used) = stats.last_used_at {
                if last_used < cutoff_time {
                    continue;
                }
            }

            // Get reality ratio (default to 0.0 if not found)
            let reality_ratio = reality_ratios
                .get(&format!("{} {}", stats.method, stats.endpoint))
                .copied()
                .unwrap_or(0.0);

            // Skip endpoints with high reality ratio if configured
            if !self.config.sync_real_endpoints && reality_ratio > 0.7 {
                continue;
            }

            // Calculate recency score (0.0 to 1.0)
            let recency_score = if let Some(last_used) = stats.last_used_at {
                let age_seconds = (now - last_used).num_seconds().max(0) as f64;
                let lookback_seconds = lookback_duration.num_seconds() as f64;
                (1.0 - (age_seconds / lookback_seconds)).max(0.0).min(1.0)
            } else {
                0.0
            };

            // Calculate priority score
            let score = (stats.total_requests as f64 * self.config.weight_count)
                + (recency_score * self.config.weight_recency)
                + (reality_ratio * self.config.weight_reality);

            priorities.push(EndpointPriority {
                endpoint: stats.endpoint.clone(),
                method: stats.method.clone(),
                score,
                request_count: stats.total_requests,
                recency_score,
                reality_ratio,
            });
        }

        // Sort by priority score (descending)
        priorities
            .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        priorities
    }

    /// Filter changes based on traffic-aware configuration
    pub fn filter_changes(
        &self,
        changes: &[DetectedChange],
        priorities: &[EndpointPriority],
    ) -> Vec<DetectedChange> {
        if !self.config.enabled {
            return changes.to_vec();
        }

        // Create a set of prioritized endpoints
        let mut prioritized_endpoints = std::collections::HashSet::new();

        // Apply threshold filters
        let filtered_priorities: Vec<&EndpointPriority> =
            if let Some(min_requests) = self.config.min_requests_threshold {
                priorities.iter().filter(|p| p.request_count >= min_requests as u64).collect()
            } else {
                priorities.iter().collect()
            };

        // Apply top percentage filter
        let selected_priorities: Vec<&EndpointPriority> =
            if let Some(top_pct) = self.config.top_percentage {
                let count =
                    ((filtered_priorities.len() as f64 * top_pct / 100.0).ceil() as usize).max(1);
                filtered_priorities.into_iter().take(count).collect()
            } else {
                filtered_priorities
            };

        // Build set of endpoints to sync
        for priority in selected_priorities {
            prioritized_endpoints.insert(format!("{} {}", priority.method, priority.endpoint));
        }

        // Filter changes to only include prioritized endpoints
        changes
            .iter()
            .filter(|change| {
                let key = format!("{} {}", change.method, change.path);
                prioritized_endpoints.contains(&key)
            })
            .cloned()
            .collect()
    }

    /// Get reality ratios for endpoints from Reality Continuum engine
    pub async fn get_reality_ratios(
        &self,
        endpoints: &[(&str, &str)],
        continuum_engine: Option<
            &mockforge_core::reality_continuum::engine::RealityContinuumEngine,
        >,
    ) -> HashMap<String, f64> {
        let mut ratios = HashMap::new();

        if let Some(engine) = continuum_engine {
            for (method, endpoint) in endpoints {
                let ratio = engine.get_blend_ratio(endpoint).await;
                ratios.insert(format!("{} {}", method, endpoint), ratio);
            }
        } else {
            // Default to 0.0 (all mock) if no continuum engine
            for (method, endpoint) in endpoints {
                ratios.insert(format!("{} {}", method, endpoint), 0.0);
            }
        }

        ratios
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::RecorderDatabase;
    use crate::models::{Protocol, RecordedRequest};

    fn create_test_summary() -> crate::diff::ComparisonSummary {
        crate::diff::ComparisonSummary {
            total_differences: 0,
            added_fields: 0,
            removed_fields: 0,
            changed_fields: 0,
            type_changes: 0,
        }
    }

    fn create_test_comparison_result(matches: bool) -> crate::diff::ComparisonResult {
        crate::diff::ComparisonResult {
            matches,
            status_match: matches,
            headers_match: matches,
            body_match: matches,
            differences: vec![],
            summary: create_test_summary(),
        }
    }

    fn create_test_traffic_config() -> TrafficAwareConfig {
        TrafficAwareConfig {
            enabled: true,
            min_requests_threshold: Some(5),
            top_percentage: Some(50.0),
            lookback_days: 7,
            sync_real_endpoints: false,
            weight_count: 1.0,
            weight_recency: 0.5,
            weight_reality: -0.3,
        }
    }

    fn create_test_usage_stats() -> HashMap<String, EndpointUsageStats> {
        let mut stats = HashMap::new();

        stats.insert(
            "GET /api/users".to_string(),
            EndpointUsageStats {
                endpoint: "/api/users".to_string(),
                method: "GET".to_string(),
                total_requests: 100,
                last_used_at: Some(Utc::now() - Duration::hours(1)),
                unique_consumers: 5,
            },
        );

        stats.insert(
            "POST /api/posts".to_string(),
            EndpointUsageStats {
                endpoint: "/api/posts".to_string(),
                method: "POST".to_string(),
                total_requests: 50,
                last_used_at: Some(Utc::now() - Duration::days(2)),
                unique_consumers: 3,
            },
        );

        stats.insert(
            "DELETE /api/old".to_string(),
            EndpointUsageStats {
                endpoint: "/api/old".to_string(),
                method: "DELETE".to_string(),
                total_requests: 10,
                last_used_at: Some(Utc::now() - Duration::days(10)),
                unique_consumers: 1,
            },
        );

        stats
    }

    #[test]
    fn test_traffic_analyzer_creation() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        // Should create successfully
        assert!(std::mem::size_of_val(&analyzer) > 0);
    }

    #[tokio::test]
    async fn test_aggregate_usage_stats_empty_db() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        let database = RecorderDatabase::new_in_memory().await.unwrap();
        let stats = analyzer.aggregate_usage_stats_from_db(&database).await;

        assert!(stats.is_empty());
    }

    #[tokio::test]
    async fn test_aggregate_usage_stats_with_requests() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        let database = RecorderDatabase::new_in_memory().await.unwrap();

        // Insert test requests
        for i in 0..5 {
            let request = RecordedRequest {
                id: format!("req-{}", i),
                protocol: Protocol::Http,
                timestamp: Utc::now(),
                method: "GET".to_string(),
                path: "/api/users".to_string(),
                query_params: None,
                headers: "{}".to_string(),
                body: None,
                body_encoding: "utf8".to_string(),
                client_ip: None,
                trace_id: None,
                span_id: None,
                duration_ms: Some(100),
                status_code: Some(200),
                tags: None,
            };
            database.insert_request(&request).await.unwrap();
        }

        let stats = analyzer.aggregate_usage_stats_from_db(&database).await;

        assert_eq!(stats.len(), 1);
        let user_stats = stats.get("GET /api/users").unwrap();
        assert_eq!(user_stats.total_requests, 5);
        assert_eq!(user_stats.endpoint, "/api/users");
        assert_eq!(user_stats.method, "GET");
    }

    #[test]
    fn test_calculate_priorities_basic() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        let usage_stats = create_test_usage_stats();
        let reality_ratios = HashMap::new();

        let priorities = analyzer.calculate_priorities(&usage_stats, &reality_ratios);

        // Should return priorities sorted by score
        assert!(!priorities.is_empty());
        assert_eq!(priorities[0].endpoint, "/api/users");
        assert!(priorities[0].score > 0.0);
    }

    #[test]
    fn test_calculate_priorities_with_reality_ratios() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        let usage_stats = create_test_usage_stats();
        let mut reality_ratios = HashMap::new();
        reality_ratios.insert("GET /api/users".to_string(), 0.5);
        reality_ratios.insert("POST /api/posts".to_string(), 0.2);

        let priorities = analyzer.calculate_priorities(&usage_stats, &reality_ratios);

        assert!(!priorities.is_empty());
        for priority in &priorities {
            if priority.endpoint == "/api/users" {
                assert_eq!(priority.reality_ratio, 0.5);
            }
        }
    }

    #[test]
    fn test_calculate_priorities_filters_old_endpoints() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        let usage_stats = create_test_usage_stats();
        let reality_ratios = HashMap::new();

        let priorities = analyzer.calculate_priorities(&usage_stats, &reality_ratios);

        // Old endpoint (10 days ago, beyond lookback window) should be filtered
        let has_old = priorities.iter().any(|p| p.endpoint == "/api/old");
        assert!(!has_old);
    }

    #[test]
    fn test_calculate_priorities_filters_high_reality_ratio() {
        let mut config = create_test_traffic_config();
        config.sync_real_endpoints = false;
        let analyzer = TrafficAnalyzer::new(config);

        let usage_stats = create_test_usage_stats();
        let mut reality_ratios = HashMap::new();
        reality_ratios.insert("GET /api/users".to_string(), 0.9); // High reality ratio

        let priorities = analyzer.calculate_priorities(&usage_stats, &reality_ratios);

        // Endpoint with high reality ratio should be filtered
        let has_high_reality = priorities.iter().any(|p| p.endpoint == "/api/users");
        assert!(!has_high_reality);
    }

    #[test]
    fn test_calculate_priorities_includes_high_reality_when_enabled() {
        let mut config = create_test_traffic_config();
        config.sync_real_endpoints = true;
        let analyzer = TrafficAnalyzer::new(config);

        let usage_stats = create_test_usage_stats();
        let mut reality_ratios = HashMap::new();
        reality_ratios.insert("GET /api/users".to_string(), 0.9);

        let priorities = analyzer.calculate_priorities(&usage_stats, &reality_ratios);

        // Should include high reality ratio when enabled
        let has_high_reality = priorities.iter().any(|p| p.endpoint == "/api/users");
        assert!(has_high_reality);
    }

    #[test]
    fn test_calculate_priorities_recency_score() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        let usage_stats = create_test_usage_stats();
        let reality_ratios = HashMap::new();

        let priorities = analyzer.calculate_priorities(&usage_stats, &reality_ratios);

        // More recent endpoint should have higher recency score
        let users_priority = priorities.iter().find(|p| p.endpoint == "/api/users").unwrap();
        let posts_priority = priorities.iter().find(|p| p.endpoint == "/api/posts").unwrap();

        assert!(users_priority.recency_score > posts_priority.recency_score);
    }

    #[test]
    fn test_filter_changes_disabled() {
        let mut config = create_test_traffic_config();
        config.enabled = false;
        let analyzer = TrafficAnalyzer::new(config);

        let changes = vec![
            crate::sync::DetectedChange {
                request_id: "req-1".to_string(),
                method: "GET".to_string(),
                path: "/api/users".to_string(),
                comparison: create_test_comparison_result(false),
                updated: false,
            },
            crate::sync::DetectedChange {
                request_id: "req-2".to_string(),
                method: "POST".to_string(),
                path: "/api/posts".to_string(),
                comparison: create_test_comparison_result(false),
                updated: false,
            },
        ];

        let priorities = vec![];
        let filtered = analyzer.filter_changes(&changes, &priorities);

        // Should return all changes when disabled
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_changes_with_min_requests() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        let changes = vec![
            crate::sync::DetectedChange {
                request_id: "req-1".to_string(),
                method: "GET".to_string(),
                path: "/api/users".to_string(),
                comparison: create_test_comparison_result(false),
                updated: false,
            },
            crate::sync::DetectedChange {
                request_id: "req-2".to_string(),
                method: "POST".to_string(),
                path: "/api/posts".to_string(),
                comparison: create_test_comparison_result(false),
                updated: false,
            },
        ];

        let priorities = vec![
            EndpointPriority {
                endpoint: "/api/users".to_string(),
                method: "GET".to_string(),
                score: 100.0,
                request_count: 100,
                recency_score: 0.9,
                reality_ratio: 0.0,
            },
            EndpointPriority {
                endpoint: "/api/posts".to_string(),
                method: "POST".to_string(),
                score: 50.0,
                request_count: 3, // Below min_requests_threshold of 5
                recency_score: 0.5,
                reality_ratio: 0.0,
            },
        ];

        let filtered = analyzer.filter_changes(&changes, &priorities);

        // Only /api/users should pass the threshold
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].path, "/api/users");
    }

    #[test]
    fn test_filter_changes_with_top_percentage() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        let changes = vec![
            crate::sync::DetectedChange {
                request_id: "req-1".to_string(),
                method: "GET".to_string(),
                path: "/api/users".to_string(),
                comparison: create_test_comparison_result(false),
                updated: false,
            },
            crate::sync::DetectedChange {
                request_id: "req-2".to_string(),
                method: "POST".to_string(),
                path: "/api/posts".to_string(),
                comparison: create_test_comparison_result(false),
                updated: false,
            },
        ];

        let priorities = vec![
            EndpointPriority {
                endpoint: "/api/users".to_string(),
                method: "GET".to_string(),
                score: 100.0,
                request_count: 100,
                recency_score: 0.9,
                reality_ratio: 0.0,
            },
            EndpointPriority {
                endpoint: "/api/posts".to_string(),
                method: "POST".to_string(),
                score: 50.0,
                request_count: 50,
                recency_score: 0.5,
                reality_ratio: 0.0,
            },
        ];

        let filtered = analyzer.filter_changes(&changes, &priorities);

        // With 50% top percentage, should get top 1 endpoint
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].path, "/api/users");
    }

    #[test]
    fn test_filter_changes_empty_priorities() {
        let config = create_test_traffic_config();
        let analyzer = TrafficAnalyzer::new(config);

        let changes = vec![crate::sync::DetectedChange {
            request_id: "req-1".to_string(),
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            comparison: create_test_comparison_result(false),
            updated: false,
        }];

        let priorities = vec![];
        let filtered = analyzer.filter_changes(&changes, &priorities);

        // No priorities means nothing passes filter
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_endpoint_usage_stats_creation() {
        let stats = EndpointUsageStats {
            endpoint: "/api/test".to_string(),
            method: "GET".to_string(),
            total_requests: 42,
            last_used_at: Some(Utc::now()),
            unique_consumers: 3,
        };

        assert_eq!(stats.endpoint, "/api/test");
        assert_eq!(stats.method, "GET");
        assert_eq!(stats.total_requests, 42);
        assert_eq!(stats.unique_consumers, 3);
    }

    #[test]
    fn test_endpoint_priority_creation() {
        let priority = EndpointPriority {
            endpoint: "/api/test".to_string(),
            method: "POST".to_string(),
            score: 75.5,
            request_count: 100,
            recency_score: 0.8,
            reality_ratio: 0.3,
        };

        assert_eq!(priority.endpoint, "/api/test");
        assert_eq!(priority.method, "POST");
        assert_eq!(priority.score, 75.5);
        assert_eq!(priority.request_count, 100);
        assert_eq!(priority.recency_score, 0.8);
        assert_eq!(priority.reality_ratio, 0.3);
    }

    #[test]
    fn test_traffic_aware_config_serialization() {
        let config = create_test_traffic_config();
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("enabled"));
        assert!(json.contains("min_requests_threshold"));
        assert!(json.contains("top_percentage"));
    }

    #[test]
    fn test_traffic_aware_config_deserialization() {
        let json = r#"{
            "enabled": true,
            "min_requests_threshold": 10,
            "top_percentage": 75.0,
            "lookback_days": 14,
            "sync_real_endpoints": true,
            "weight_count": 2.0,
            "weight_recency": 1.0,
            "weight_reality": 0.5
        }"#;

        let config: TrafficAwareConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.min_requests_threshold, Some(10));
        assert_eq!(config.top_percentage, Some(75.0));
        assert_eq!(config.lookback_days, 14);
        assert!(config.sync_real_endpoints);
    }

    #[test]
    fn test_traffic_aware_config_defaults() {
        let json = r#"{
            "enabled": true
        }"#;

        let config: TrafficAwareConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.lookback_days, 7);
        assert_eq!(config.weight_count, 1.0);
        assert_eq!(config.weight_recency, 0.5);
        assert_eq!(config.weight_reality, -0.3);
        assert!(!config.sync_real_endpoints);
    }
}
