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
        let cutoff_time = Utc::now() - chrono::Duration::days(self.config.lookback_days as i64);

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
                    if stats.last_used_at.map_or(true, |last| rt > last) {
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

        for (_key, stats) in usage_stats {
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
