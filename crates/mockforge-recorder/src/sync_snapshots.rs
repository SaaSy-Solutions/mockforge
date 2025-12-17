//! Shadow Snapshot Mode - Store canonical before/after datasets per endpoint scenario
//!
//! This module provides functionality to store snapshots of API responses before and after
//! sync operations, enabling timeline visualization of how endpoints evolve over time.

use crate::diff::ComparisonResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Snapshot data for a single point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body as raw bytes
    pub body: Vec<u8>,
    /// Response body as parsed JSON (if applicable)
    pub body_json: Option<Value>,
}

/// Sync snapshot capturing before/after state for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSnapshot {
    /// Unique snapshot ID
    pub id: String,
    /// Endpoint path (e.g., "/api/users/{id}")
    pub endpoint: String,
    /// HTTP method (e.g., "GET", "POST")
    pub method: String,
    /// Sync cycle ID (groups snapshots from the same sync operation)
    pub sync_cycle_id: String,
    /// Timestamp when snapshot was created
    pub timestamp: DateTime<Utc>,
    /// Original fixture data (before sync)
    pub before: SnapshotData,
    /// New upstream response (after sync)
    pub after: SnapshotData,
    /// Comparison result showing differences
    pub changes: ComparisonResult,
    /// Response time before sync (milliseconds)
    pub response_time_before: Option<u64>,
    /// Response time after sync (milliseconds)
    pub response_time_after: Option<u64>,
}

impl SyncSnapshot {
    /// Create a new sync snapshot
    pub fn new(
        endpoint: String,
        method: String,
        sync_cycle_id: String,
        before: SnapshotData,
        after: SnapshotData,
        changes: ComparisonResult,
        response_time_before: Option<u64>,
        response_time_after: Option<u64>,
    ) -> Self {
        let id = format!(
            "snapshot_{}_{}_{}",
            endpoint.replace('/', "_").replace(['{', '}'], ""),
            method.to_lowercase(),
            &uuid::Uuid::new_v4().to_string()[..8]
        );

        Self {
            id,
            endpoint,
            method,
            sync_cycle_id,
            timestamp: Utc::now(),
            before,
            after,
            changes,
            response_time_before,
            response_time_after,
        }
    }
}

/// Timeline data for visualizing endpoint evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointTimeline {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// List of snapshots in chronological order
    pub snapshots: Vec<SyncSnapshot>,
    /// Response time trends (timestamp -> response_time_ms)
    pub response_time_trends: Vec<(DateTime<Utc>, Option<u64>)>,
    /// Status code changes over time (timestamp -> status_code)
    pub status_code_history: Vec<(DateTime<Utc>, u16)>,
    /// Common error patterns detected
    pub error_patterns: Vec<ErrorPattern>,
}

/// Error pattern detected in response history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    /// Status code
    pub status_code: u16,
    /// Error message pattern (if extractable)
    pub message_pattern: Option<String>,
    /// Number of occurrences
    pub occurrences: usize,
    /// First occurrence timestamp
    pub first_seen: DateTime<Utc>,
    /// Last occurrence timestamp
    pub last_seen: DateTime<Utc>,
}

/// Summary statistics for endpoint evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointEvolutionSummary {
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Total number of snapshots
    pub total_snapshots: usize,
    /// Number of changes detected
    pub total_changes: usize,
    /// Average response time (milliseconds)
    pub avg_response_time: Option<f64>,
    /// Most common status code
    pub most_common_status: Option<u16>,
    /// Field-level change frequency (field_path -> count)
    pub field_change_frequency: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_summary(total_differences: usize) -> crate::diff::ComparisonSummary {
        crate::diff::ComparisonSummary {
            total_differences,
            added_fields: 0,
            removed_fields: 0,
            changed_fields: total_differences,
            type_changes: 0,
        }
    }

    fn create_test_comparison_result(
        matches: bool,
        differences: Vec<crate::diff::Difference>,
    ) -> crate::diff::ComparisonResult {
        crate::diff::ComparisonResult {
            matches,
            status_match: matches,
            headers_match: matches,
            body_match: matches,
            differences: differences.clone(),
            summary: create_test_summary(differences.len()),
        }
    }

    fn create_test_snapshot_data() -> SnapshotData {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        SnapshotData {
            status_code: 200,
            headers,
            body: b"test body".to_vec(),
            body_json: Some(serde_json::json!({"status": "ok"})),
        }
    }

    #[test]
    fn test_snapshot_data_creation() {
        let snapshot = create_test_snapshot_data();

        assert_eq!(snapshot.status_code, 200);
        assert_eq!(snapshot.headers.get("content-type").unwrap(), "application/json");
        assert_eq!(snapshot.body, b"test body");
        assert!(snapshot.body_json.is_some());
    }

    #[test]
    fn test_sync_snapshot_new() {
        let before = create_test_snapshot_data();
        let mut after = create_test_snapshot_data();
        after.status_code = 201;

        let differences = vec![crate::diff::Difference::new(
            "$.status_code".to_string(),
            crate::diff::DifferenceType::Changed {
                path: "$.status_code".to_string(),
                original: "200".to_string(),
                current: "201".to_string(),
            },
        )];
        let comparison = create_test_comparison_result(false, differences);

        let snapshot = SyncSnapshot::new(
            "/api/users".to_string(),
            "GET".to_string(),
            "cycle-123".to_string(),
            before.clone(),
            after.clone(),
            comparison,
            Some(100),
            Some(120),
        );

        assert!(snapshot.id.contains("snapshot"));
        assert_eq!(snapshot.endpoint, "/api/users");
        assert_eq!(snapshot.method, "GET");
        assert_eq!(snapshot.sync_cycle_id, "cycle-123");
        assert_eq!(snapshot.before.status_code, 200);
        assert_eq!(snapshot.after.status_code, 201);
        assert_eq!(snapshot.response_time_before, Some(100));
        assert_eq!(snapshot.response_time_after, Some(120));
    }

    #[test]
    fn test_sync_snapshot_id_generation() {
        let before = create_test_snapshot_data();
        let after = create_test_snapshot_data();

        let comparison = create_test_comparison_result(true, vec![]);

        let snapshot1 = SyncSnapshot::new(
            "/api/users/{id}".to_string(),
            "GET".to_string(),
            "cycle-1".to_string(),
            before.clone(),
            after.clone(),
            comparison.clone(),
            None,
            None,
        );

        let snapshot2 = SyncSnapshot::new(
            "/api/users/{id}".to_string(),
            "GET".to_string(),
            "cycle-2".to_string(),
            before.clone(),
            after.clone(),
            comparison,
            None,
            None,
        );

        // IDs should be different (contain UUID)
        assert_ne!(snapshot1.id, snapshot2.id);

        // But should have similar structure
        assert!(snapshot1.id.starts_with("snapshot_"));
        assert!(snapshot2.id.starts_with("snapshot_"));
    }

    #[test]
    fn test_sync_snapshot_serialization() {
        let before = create_test_snapshot_data();
        let after = create_test_snapshot_data();

        let comparison = create_test_comparison_result(true, vec![]);

        let snapshot = SyncSnapshot::new(
            "/api/test".to_string(),
            "POST".to_string(),
            "cycle-abc".to_string(),
            before,
            after,
            comparison,
            Some(50),
            Some(55),
        );

        let json = serde_json::to_string(&snapshot).unwrap();

        assert!(json.contains("/api/test"));
        assert!(json.contains("POST"));
        assert!(json.contains("cycle-abc"));
    }

    #[test]
    fn test_sync_snapshot_deserialization() {
        let before = create_test_snapshot_data();
        let after = create_test_snapshot_data();

        let comparison = create_test_comparison_result(true, vec![]);

        let original = SyncSnapshot::new(
            "/api/test".to_string(),
            "GET".to_string(),
            "cycle-xyz".to_string(),
            before,
            after,
            comparison,
            Some(100),
            Some(105),
        );

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: SyncSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.endpoint, original.endpoint);
        assert_eq!(deserialized.method, original.method);
        assert_eq!(deserialized.sync_cycle_id, original.sync_cycle_id);
        assert_eq!(deserialized.response_time_before, original.response_time_before);
        assert_eq!(deserialized.response_time_after, original.response_time_after);
    }

    #[test]
    fn test_endpoint_timeline_creation() {
        let snapshots = vec![];

        let timeline = EndpointTimeline {
            endpoint: "/api/users".to_string(),
            method: "GET".to_string(),
            snapshots,
            response_time_trends: vec![],
            status_code_history: vec![],
            error_patterns: vec![],
        };

        assert_eq!(timeline.endpoint, "/api/users");
        assert_eq!(timeline.method, "GET");
        assert!(timeline.snapshots.is_empty());
    }

    #[test]
    fn test_endpoint_timeline_with_snapshots() {
        let before = create_test_snapshot_data();
        let after = create_test_snapshot_data();

        let comparison = create_test_comparison_result(true, vec![]);

        let snapshot = SyncSnapshot::new(
            "/api/users".to_string(),
            "GET".to_string(),
            "cycle-1".to_string(),
            before,
            after,
            comparison,
            Some(100),
            Some(100),
        );

        let now = Utc::now();
        let response_time_trends = vec![(now, Some(100))];
        let status_code_history = vec![(now, 200)];

        let timeline = EndpointTimeline {
            endpoint: "/api/users".to_string(),
            method: "GET".to_string(),
            snapshots: vec![snapshot],
            response_time_trends,
            status_code_history,
            error_patterns: vec![],
        };

        assert_eq!(timeline.snapshots.len(), 1);
        assert_eq!(timeline.response_time_trends.len(), 1);
        assert_eq!(timeline.status_code_history.len(), 1);
    }

    #[test]
    fn test_error_pattern_creation() {
        let now = Utc::now();

        let pattern = ErrorPattern {
            status_code: 404,
            message_pattern: Some("Not found".to_string()),
            occurrences: 5,
            first_seen: now,
            last_seen: now,
        };

        assert_eq!(pattern.status_code, 404);
        assert_eq!(pattern.message_pattern, Some("Not found".to_string()));
        assert_eq!(pattern.occurrences, 5);
    }

    #[test]
    fn test_error_pattern_serialization() {
        let now = Utc::now();

        let pattern = ErrorPattern {
            status_code: 500,
            message_pattern: Some("Internal server error".to_string()),
            occurrences: 3,
            first_seen: now,
            last_seen: now,
        };

        let json = serde_json::to_string(&pattern).unwrap();

        assert!(json.contains("500"));
        assert!(json.contains("Internal server error"));
        assert!(json.contains("3"));
    }

    #[test]
    fn test_endpoint_evolution_summary_creation() {
        let mut field_change_frequency = HashMap::new();
        field_change_frequency.insert("$.user.name".to_string(), 5);
        field_change_frequency.insert("$.user.email".to_string(), 3);

        let summary = EndpointEvolutionSummary {
            endpoint: "/api/users".to_string(),
            method: "GET".to_string(),
            total_snapshots: 10,
            total_changes: 8,
            avg_response_time: Some(125.5),
            most_common_status: Some(200),
            field_change_frequency,
        };

        assert_eq!(summary.endpoint, "/api/users");
        assert_eq!(summary.method, "GET");
        assert_eq!(summary.total_snapshots, 10);
        assert_eq!(summary.total_changes, 8);
        assert_eq!(summary.avg_response_time, Some(125.5));
        assert_eq!(summary.most_common_status, Some(200));
        assert_eq!(summary.field_change_frequency.len(), 2);
    }

    #[test]
    fn test_endpoint_evolution_summary_serialization() {
        let mut field_change_frequency = HashMap::new();
        field_change_frequency.insert("$.status".to_string(), 2);

        let summary = EndpointEvolutionSummary {
            endpoint: "/api/test".to_string(),
            method: "POST".to_string(),
            total_snapshots: 5,
            total_changes: 3,
            avg_response_time: Some(100.0),
            most_common_status: Some(201),
            field_change_frequency,
        };

        let json = serde_json::to_string(&summary).unwrap();

        assert!(json.contains("/api/test"));
        assert!(json.contains("POST"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_snapshot_data_with_no_json() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/plain".to_string());

        let snapshot = SnapshotData {
            status_code: 200,
            headers,
            body: b"plain text".to_vec(),
            body_json: None,
        };

        assert!(snapshot.body_json.is_none());
        assert_eq!(snapshot.body, b"plain text");
    }

    #[test]
    fn test_sync_snapshot_with_no_response_times() {
        let before = create_test_snapshot_data();
        let after = create_test_snapshot_data();

        let comparison = create_test_comparison_result(true, vec![]);

        let snapshot = SyncSnapshot::new(
            "/api/test".to_string(),
            "GET".to_string(),
            "cycle-1".to_string(),
            before,
            after,
            comparison,
            None,
            None,
        );

        assert_eq!(snapshot.response_time_before, None);
        assert_eq!(snapshot.response_time_after, None);
    }

    #[test]
    fn test_endpoint_timeline_serialization() {
        let timeline = EndpointTimeline {
            endpoint: "/api/users".to_string(),
            method: "GET".to_string(),
            snapshots: vec![],
            response_time_trends: vec![],
            status_code_history: vec![],
            error_patterns: vec![],
        };

        let json = serde_json::to_string(&timeline).unwrap();

        assert!(json.contains("/api/users"));
        assert!(json.contains("GET"));
    }

    #[test]
    fn test_snapshot_data_clone() {
        let snapshot = create_test_snapshot_data();
        let cloned = snapshot.clone();

        assert_eq!(snapshot.status_code, cloned.status_code);
        assert_eq!(snapshot.body, cloned.body);
    }

    #[test]
    fn test_sync_snapshot_clone() {
        let before = create_test_snapshot_data();
        let after = create_test_snapshot_data();

        let comparison = create_test_comparison_result(true, vec![]);

        let snapshot = SyncSnapshot::new(
            "/api/test".to_string(),
            "GET".to_string(),
            "cycle-1".to_string(),
            before,
            after,
            comparison,
            Some(100),
            Some(100),
        );

        let cloned = snapshot.clone();

        assert_eq!(snapshot.id, cloned.id);
        assert_eq!(snapshot.endpoint, cloned.endpoint);
        assert_eq!(snapshot.method, cloned.method);
    }

    #[test]
    fn test_error_pattern_clone() {
        let now = Utc::now();

        let pattern = ErrorPattern {
            status_code: 404,
            message_pattern: Some("Not found".to_string()),
            occurrences: 5,
            first_seen: now,
            last_seen: now,
        };

        let cloned = pattern.clone();

        assert_eq!(pattern.status_code, cloned.status_code);
        assert_eq!(pattern.message_pattern, cloned.message_pattern);
        assert_eq!(pattern.occurrences, cloned.occurrences);
    }
}
