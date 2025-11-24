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
