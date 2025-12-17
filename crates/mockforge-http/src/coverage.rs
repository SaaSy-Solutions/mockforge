//! Mock Coverage Tracking
//!
//! This module provides API coverage tracking functionality, allowing users to see
//! which endpoints from their OpenAPI spec have been exercised during testing.
//! This is analogous to code coverage but for API surface area.
use axum::{
    extract::{Query, State},
    response::Json,
};
use mockforge_observability::prometheus::{get_global_registry, MetricFamily};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{HttpServerState, RouteInfo};

/// Coverage information for a single route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteCoverage {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Route path template
    pub path: String,
    /// Operation ID from OpenAPI spec
    pub operation_id: Option<String>,
    /// Operation summary
    pub summary: Option<String>,
    /// Whether this route has been called
    pub covered: bool,
    /// Number of times this route has been called
    pub hit_count: u64,
    /// Breakdown by status code
    pub status_breakdown: HashMap<u16, u64>,
    /// Average latency in seconds (if called)
    pub avg_latency_seconds: Option<f64>,
}

/// Overall coverage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Total number of routes defined in the spec
    pub total_routes: usize,
    /// Number of routes that have been called
    pub covered_routes: usize,
    /// Coverage percentage (0.0 to 100.0)
    pub coverage_percentage: f64,
    /// Individual route coverage details
    pub routes: Vec<RouteCoverage>,
    /// Coverage breakdown by HTTP method
    pub method_coverage: HashMap<String, MethodCoverage>,
    /// Timestamp of the report
    pub timestamp: String,
}

/// Coverage statistics for a specific HTTP method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodCoverage {
    /// Total number of routes for this HTTP method
    pub total: usize,
    /// Number of routes that have been covered (called at least once)
    pub covered: usize,
    /// Coverage percentage (0.0 to 100.0)
    pub percentage: f64,
}

/// Query parameters for coverage endpoint
#[derive(Debug, Deserialize)]
pub struct CoverageQuery {
    /// Filter by HTTP method (e.g., "GET", "POST")
    pub method: Option<String>,
    /// Filter by path pattern (e.g., "/users")
    pub path: Option<String>,
    /// Only show uncovered routes
    pub uncovered_only: Option<bool>,
}

/// Calculate coverage for all routes
pub async fn calculate_coverage(routes: &[RouteInfo]) -> CoverageReport {
    let metrics_registry = get_global_registry();

    // Gather metrics from Prometheus
    let metric_families = metrics_registry.registry().gather();
    let path_metrics = extract_path_metrics(&metric_families);

    let mut route_coverages = Vec::new();
    let mut covered_count = 0;
    let mut method_stats: HashMap<String, (usize, usize)> = HashMap::new();

    for route in routes {
        let normalized_path = normalize_path(&route.path);
        let key = format!("{} {}", route.method, normalized_path);

        // Check if this route has been hit
        let (covered, hit_count, status_breakdown) = if let Some(metrics) = path_metrics.get(&key) {
            let total_hits: u64 = metrics.values().sum();
            (total_hits > 0, total_hits, metrics.clone())
        } else {
            (false, 0, HashMap::new())
        };

        // Get average latency if available
        let avg_latency = if covered {
            get_average_latency(&metric_families, &normalized_path, &route.method)
        } else {
            None
        };

        if covered {
            covered_count += 1;
        }

        // Update method stats
        let method_entry = method_stats.entry(route.method.clone()).or_insert((0, 0));
        method_entry.0 += 1; // total
        if covered {
            method_entry.1 += 1; // covered
        }

        route_coverages.push(RouteCoverage {
            method: route.method.clone(),
            path: route.path.clone(),
            operation_id: route.operation_id.clone(),
            summary: route.summary.clone(),
            covered,
            hit_count,
            status_breakdown,
            avg_latency_seconds: avg_latency,
        });
    }

    let total_routes = routes.len();
    let coverage_percentage = if total_routes > 0 {
        (covered_count as f64 / total_routes as f64) * 100.0
    } else {
        0.0
    };

    // Build method coverage breakdown
    let method_coverage = method_stats
        .into_iter()
        .map(|(method, (total, covered))| {
            let percentage = if total > 0 {
                (covered as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            (
                method,
                MethodCoverage {
                    total,
                    covered,
                    percentage,
                },
            )
        })
        .collect();

    CoverageReport {
        total_routes,
        covered_routes: covered_count,
        coverage_percentage,
        routes: route_coverages,
        method_coverage,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

/// Extract path-based metrics from Prometheus metric families
fn extract_path_metrics(metric_families: &[MetricFamily]) -> HashMap<String, HashMap<u16, u64>> {
    let mut path_metrics: HashMap<String, HashMap<u16, u64>> = HashMap::new();

    // Find the requests_by_path_total metric
    for mf in metric_families {
        if mf.name() == "mockforge_requests_by_path_total" {
            for metric in mf.get_metric() {
                let mut path = String::new();
                let mut method = String::new();
                let mut status = 0u16;

                // Extract labels
                for label_pair in metric.get_label() {
                    match label_pair.name() {
                        "path" => path = label_pair.value().to_string(),
                        "method" => method = label_pair.value().to_string(),
                        "status" => {
                            status = label_pair.value().parse().unwrap_or(0);
                        }
                        _ => {}
                    }
                }

                let key = format!("{} {}", method, path);
                let count = metric.get_counter().value.unwrap_or(0.0) as u64;

                path_metrics.entry(key).or_default().insert(status, count);
            }
        }
    }

    path_metrics
}

/// Get average latency for a specific route
fn get_average_latency(metric_families: &[MetricFamily], path: &str, method: &str) -> Option<f64> {
    for mf in metric_families {
        if mf.name() == "mockforge_average_latency_by_path_seconds" {
            for metric in mf.get_metric() {
                let mut metric_path = String::new();
                let mut metric_method = String::new();

                for label_pair in metric.get_label() {
                    match label_pair.name() {
                        "path" => metric_path = label_pair.value().to_string(),
                        "method" => metric_method = label_pair.value().to_string(),
                        _ => {}
                    }
                }

                if metric_path == path && metric_method == method {
                    if let Some(value) = metric.get_gauge().value {
                        return if value > 0.0 { Some(value) } else { None };
                    }
                }
            }
        }
    }

    None
}

/// Normalize path to match metrics normalization
/// This must match the logic in mockforge-observability/src/prometheus/metrics.rs
fn normalize_path(path: &str) -> String {
    let mut segments: Vec<&str> = path.split('/').collect();

    for segment in &mut segments {
        // Replace path parameters like {id} with :id
        if segment.starts_with('{') && segment.ends_with('}')
            || is_uuid(segment)
            || segment.parse::<i64>().is_ok()
            || (segment.len() > 8 && segment.chars().all(|c| c.is_ascii_hexdigit()))
        {
            *segment = ":id";
        }
    }

    segments.join("/")
}

/// Check if a string is a UUID
fn is_uuid(s: &str) -> bool {
    s.len() == 36 && s.chars().filter(|&c| c == '-').count() == 4
}

/// Handler for the coverage endpoint
pub async fn get_coverage_handler(
    State(state): State<HttpServerState>,
    Query(params): Query<CoverageQuery>,
) -> Json<CoverageReport> {
    let mut report = calculate_coverage(&state.routes).await;

    // Apply filters
    if let Some(method_filter) = params.method {
        report.routes.retain(|r| r.method == method_filter);
        report.total_routes = report.routes.len();
        report.covered_routes = report.routes.iter().filter(|r| r.covered).count();
        report.coverage_percentage = if report.total_routes > 0 {
            (report.covered_routes as f64 / report.total_routes as f64) * 100.0
        } else {
            0.0
        };
    }

    if let Some(path_filter) = params.path {
        report.routes.retain(|r| r.path.contains(&path_filter));
        report.total_routes = report.routes.len();
        report.covered_routes = report.routes.iter().filter(|r| r.covered).count();
        report.coverage_percentage = if report.total_routes > 0 {
            (report.covered_routes as f64 / report.total_routes as f64) * 100.0
        } else {
            0.0
        };
    }

    if params.uncovered_only.unwrap_or(false) {
        report.routes.retain(|r| !r.covered);
        report.total_routes = report.routes.len();
        report.covered_routes = 0;
        report.coverage_percentage = 0.0;
    }

    Json(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Normalize Path Tests ====================

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/users/{id}"), "/users/:id");
        assert_eq!(normalize_path("/users/123"), "/users/:id");
        assert_eq!(normalize_path("/users/550e8400-e29b-41d4-a716-446655440000"), "/users/:id");
        assert_eq!(normalize_path("/users/list"), "/users/list");
        assert_eq!(
            normalize_path("/api/v1/users/{id}/posts/{postId}"),
            "/api/v1/users/:id/posts/:id"
        );
    }

    #[test]
    fn test_normalize_path_root() {
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_normalize_path_deep_nested() {
        assert_eq!(
            normalize_path("/api/v1/org/{orgId}/team/{teamId}/member/{memberId}"),
            "/api/v1/org/:id/team/:id/member/:id"
        );
    }

    #[test]
    fn test_normalize_path_no_params() {
        assert_eq!(normalize_path("/health/check"), "/health/check");
        assert_eq!(normalize_path("/api/v1/status"), "/api/v1/status");
    }

    #[test]
    fn test_normalize_path_hex_ids() {
        // Long hex strings should be normalized
        assert_eq!(normalize_path("/objects/abcdef1234567890"), "/objects/:id");
    }

    // ==================== Is UUID Tests ====================

    #[test]
    fn test_is_uuid() {
        assert!(is_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_uuid("not-a-uuid"));
        assert!(!is_uuid("123"));
    }

    #[test]
    fn test_is_uuid_various_formats() {
        // Valid UUIDs
        assert!(is_uuid("00000000-0000-0000-0000-000000000000"));
        assert!(is_uuid("ffffffff-ffff-ffff-ffff-ffffffffffff"));
        assert!(is_uuid("12345678-1234-1234-1234-123456789abc"));

        // Invalid
        assert!(!is_uuid("12345678-1234-1234-1234-123456789")); // too short
        assert!(!is_uuid("12345678123412341234123456789abc")); // no dashes
        assert!(!is_uuid("")); // empty
    }

    // ==================== RouteCoverage Tests ====================

    #[test]
    fn test_route_coverage_creation() {
        let coverage = RouteCoverage {
            method: "GET".to_string(),
            path: "/users".to_string(),
            operation_id: Some("getUsers".to_string()),
            summary: Some("List users".to_string()),
            covered: true,
            hit_count: 100,
            status_breakdown: HashMap::new(),
            avg_latency_seconds: Some(0.05),
        };

        assert_eq!(coverage.method, "GET");
        assert!(coverage.covered);
        assert_eq!(coverage.hit_count, 100);
    }

    #[test]
    fn test_route_coverage_serialization() {
        let coverage = RouteCoverage {
            method: "POST".to_string(),
            path: "/orders".to_string(),
            operation_id: None,
            summary: None,
            covered: false,
            hit_count: 0,
            status_breakdown: HashMap::new(),
            avg_latency_seconds: None,
        };

        let json = serde_json::to_string(&coverage).unwrap();
        assert!(json.contains("POST"));
        assert!(json.contains("/orders"));
        assert!(json.contains("false"));
    }

    #[test]
    fn test_route_coverage_with_status_breakdown() {
        let mut status_breakdown = HashMap::new();
        status_breakdown.insert(200, 50);
        status_breakdown.insert(201, 30);
        status_breakdown.insert(500, 5);

        let coverage = RouteCoverage {
            method: "POST".to_string(),
            path: "/api/data".to_string(),
            operation_id: Some("createData".to_string()),
            summary: None,
            covered: true,
            hit_count: 85,
            status_breakdown,
            avg_latency_seconds: Some(0.1),
        };

        assert_eq!(coverage.status_breakdown.len(), 3);
        assert_eq!(coverage.status_breakdown.get(&200), Some(&50));
    }

    #[test]
    fn test_route_coverage_clone() {
        let coverage = RouteCoverage {
            method: "DELETE".to_string(),
            path: "/items/{id}".to_string(),
            operation_id: None,
            summary: None,
            covered: true,
            hit_count: 10,
            status_breakdown: HashMap::new(),
            avg_latency_seconds: None,
        };

        let cloned = coverage.clone();
        assert_eq!(cloned.method, coverage.method);
        assert_eq!(cloned.hit_count, coverage.hit_count);
    }

    // ==================== CoverageReport Tests ====================

    #[test]
    fn test_coverage_report_creation() {
        let report = CoverageReport {
            total_routes: 10,
            covered_routes: 7,
            coverage_percentage: 70.0,
            routes: vec![],
            method_coverage: HashMap::new(),
            timestamp: "2024-01-15T10:00:00Z".to_string(),
        };

        assert_eq!(report.total_routes, 10);
        assert_eq!(report.covered_routes, 7);
        assert_eq!(report.coverage_percentage, 70.0);
    }

    #[test]
    fn test_coverage_report_serialization() {
        let report = CoverageReport {
            total_routes: 5,
            covered_routes: 3,
            coverage_percentage: 60.0,
            routes: vec![],
            method_coverage: HashMap::new(),
            timestamp: "2024-01-15T10:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("60.0"));
        assert!(json.contains("total_routes"));
    }

    #[test]
    fn test_coverage_report_clone() {
        let report = CoverageReport {
            total_routes: 20,
            covered_routes: 15,
            coverage_percentage: 75.0,
            routes: vec![],
            method_coverage: HashMap::new(),
            timestamp: "2024-01-15T10:00:00Z".to_string(),
        };

        let cloned = report.clone();
        assert_eq!(cloned.total_routes, report.total_routes);
        assert_eq!(cloned.coverage_percentage, report.coverage_percentage);
    }

    // ==================== MethodCoverage Tests ====================

    #[test]
    fn test_method_coverage_creation() {
        let coverage = MethodCoverage {
            total: 10,
            covered: 8,
            percentage: 80.0,
        };

        assert_eq!(coverage.total, 10);
        assert_eq!(coverage.covered, 8);
        assert_eq!(coverage.percentage, 80.0);
    }

    #[test]
    fn test_method_coverage_serialization() {
        let coverage = MethodCoverage {
            total: 5,
            covered: 5,
            percentage: 100.0,
        };

        let json = serde_json::to_string(&coverage).unwrap();
        assert!(json.contains("100.0"));
    }

    #[test]
    fn test_method_coverage_clone() {
        let coverage = MethodCoverage {
            total: 3,
            covered: 2,
            percentage: 66.67,
        };

        let cloned = coverage.clone();
        assert_eq!(cloned.total, coverage.total);
    }

    // ==================== CoverageQuery Tests ====================

    #[test]
    fn test_coverage_query_empty() {
        let query = CoverageQuery {
            method: None,
            path: None,
            uncovered_only: None,
        };

        assert!(query.method.is_none());
        assert!(query.path.is_none());
        assert!(query.uncovered_only.is_none());
    }

    #[test]
    fn test_coverage_query_with_method_filter() {
        let query = CoverageQuery {
            method: Some("GET".to_string()),
            path: None,
            uncovered_only: None,
        };

        assert_eq!(query.method, Some("GET".to_string()));
    }

    #[test]
    fn test_coverage_query_with_path_filter() {
        let query = CoverageQuery {
            method: None,
            path: Some("/users".to_string()),
            uncovered_only: None,
        };

        assert_eq!(query.path, Some("/users".to_string()));
    }

    #[test]
    fn test_coverage_query_uncovered_only() {
        let query = CoverageQuery {
            method: None,
            path: None,
            uncovered_only: Some(true),
        };

        assert_eq!(query.uncovered_only, Some(true));
    }

    // ==================== Calculate Coverage Tests ====================

    #[tokio::test]
    async fn test_calculate_coverage_empty() {
        let routes = vec![];
        let report = calculate_coverage(&routes).await;

        assert_eq!(report.total_routes, 0);
        assert_eq!(report.covered_routes, 0);
        assert_eq!(report.coverage_percentage, 0.0);
    }

    #[tokio::test]
    async fn test_calculate_coverage_with_routes() {
        let routes = vec![
            RouteInfo {
                method: "GET".to_string(),
                path: "/users".to_string(),
                operation_id: Some("getUsers".to_string()),
                summary: Some("Get all users".to_string()),
                description: None,
                parameters: vec![],
            },
            RouteInfo {
                method: "POST".to_string(),
                path: "/users".to_string(),
                operation_id: Some("createUser".to_string()),
                summary: Some("Create a user".to_string()),
                description: None,
                parameters: vec![],
            },
        ];

        let report = calculate_coverage(&routes).await;

        assert_eq!(report.total_routes, 2);
        assert_eq!(report.routes.len(), 2);
        // Coverage will be 0% since no metrics have been recorded in this test
        assert!(report.coverage_percentage >= 0.0 && report.coverage_percentage <= 100.0);
    }

    #[tokio::test]
    async fn test_calculate_coverage_single_route() {
        let routes = vec![RouteInfo {
            method: "GET".to_string(),
            path: "/health".to_string(),
            operation_id: Some("healthCheck".to_string()),
            summary: None,
            description: None,
            parameters: vec![],
        }];

        let report = calculate_coverage(&routes).await;

        assert_eq!(report.total_routes, 1);
        assert_eq!(report.routes.len(), 1);
        assert_eq!(report.routes[0].method, "GET");
        assert_eq!(report.routes[0].path, "/health");
    }

    #[tokio::test]
    async fn test_calculate_coverage_method_breakdown() {
        let routes = vec![
            RouteInfo {
                method: "GET".to_string(),
                path: "/users".to_string(),
                operation_id: None,
                summary: None,
                description: None,
                parameters: vec![],
            },
            RouteInfo {
                method: "GET".to_string(),
                path: "/users/{id}".to_string(),
                operation_id: None,
                summary: None,
                description: None,
                parameters: vec![],
            },
            RouteInfo {
                method: "POST".to_string(),
                path: "/users".to_string(),
                operation_id: None,
                summary: None,
                description: None,
                parameters: vec![],
            },
        ];

        let report = calculate_coverage(&routes).await;

        assert_eq!(report.total_routes, 3);
        assert!(report.method_coverage.contains_key("GET"));
        assert!(report.method_coverage.contains_key("POST"));
        assert_eq!(report.method_coverage.get("GET").unwrap().total, 2);
        assert_eq!(report.method_coverage.get("POST").unwrap().total, 1);
    }

    // ==================== Debug Tests ====================

    #[test]
    fn test_route_coverage_debug() {
        let coverage = RouteCoverage {
            method: "GET".to_string(),
            path: "/test".to_string(),
            operation_id: None,
            summary: None,
            covered: false,
            hit_count: 0,
            status_breakdown: HashMap::new(),
            avg_latency_seconds: None,
        };

        let debug = format!("{:?}", coverage);
        assert!(debug.contains("RouteCoverage"));
    }

    #[test]
    fn test_coverage_report_debug() {
        let report = CoverageReport {
            total_routes: 0,
            covered_routes: 0,
            coverage_percentage: 0.0,
            routes: vec![],
            method_coverage: HashMap::new(),
            timestamp: "test".to_string(),
        };

        let debug = format!("{:?}", report);
        assert!(debug.contains("CoverageReport"));
    }

    #[test]
    fn test_method_coverage_debug() {
        let coverage = MethodCoverage {
            total: 5,
            covered: 3,
            percentage: 60.0,
        };

        let debug = format!("{:?}", coverage);
        assert!(debug.contains("MethodCoverage"));
    }

    #[test]
    fn test_coverage_query_debug() {
        let query = CoverageQuery {
            method: Some("GET".to_string()),
            path: None,
            uncovered_only: None,
        };

        let debug = format!("{:?}", query);
        assert!(debug.contains("CoverageQuery"));
    }
}
