//! Public snapshot, comparison and request data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Snapshot of mock server state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSnapshot {
    /// Snapshot ID
    pub id: String,
    /// Timestamp when snapshot was taken
    pub timestamp: i64,
    /// Environment ID (if applicable)
    pub environment_id: Option<String>,
    /// Persona ID (if applicable)
    pub persona_id: Option<String>,
    /// Scenario ID (if applicable)
    pub scenario_id: Option<String>,
    /// Reality level (0.0-1.0, if applicable)
    pub reality_level: Option<f64>,
    /// All mocks in this snapshot
    pub mocks: Vec<MockSnapshotItem>,
    /// Metadata about the snapshot
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Individual mock item in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSnapshotItem {
    /// Mock ID
    pub id: String,
    /// HTTP method
    pub method: String,
    /// Path pattern
    pub path: String,
    /// Response status code
    pub status_code: u16,
    /// Response body
    pub response_body: serde_json::Value,
    /// Response headers
    pub response_headers: Option<HashMap<String, String>>,
    /// Mock configuration
    pub config: serde_json::Value,
}

/// Comparison between two snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    /// Left snapshot (baseline)
    pub left: MockSnapshot,
    /// Right snapshot (comparison)
    pub right: MockSnapshot,
    /// Differences found
    pub differences: Vec<Difference>,
    /// Summary statistics
    pub summary: DiffSummary,
}

/// Individual difference between snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difference {
    /// Type of difference
    pub diff_type: DifferenceType,
    /// Mock ID (if applicable)
    pub mock_id: Option<String>,
    /// Path of the mock
    pub path: String,
    /// Method of the mock
    pub method: String,
    /// Description of the difference
    pub description: String,
    /// Left value (if applicable)
    pub left_value: Option<serde_json::Value>,
    /// Right value (if applicable)
    pub right_value: Option<serde_json::Value>,
    /// Field path where difference occurs (for nested differences)
    pub field_path: Option<String>,
}

/// Type of difference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifferenceType {
    /// Mock exists in left but not right
    MissingInRight,
    /// Mock exists in right but not left
    MissingInLeft,
    /// Status code differs
    StatusCodeMismatch,
    /// Response body differs
    BodyMismatch,
    /// Response headers differ
    HeadersMismatch,
    /// Configuration differs
    ConfigMismatch,
}

/// Summary statistics for a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Total mocks in left snapshot
    pub left_total: usize,
    /// Total mocks in right snapshot
    pub right_total: usize,
    /// Number of differences found
    pub differences_count: usize,
    /// Number of mocks only in left
    pub only_in_left: usize,
    /// Number of mocks only in right
    pub only_in_right: usize,
    /// Number of mocks with differences
    pub mocks_with_differences: usize,
}

/// Request to create a snapshot
#[derive(Debug, Deserialize)]
pub struct CreateSnapshotRequest {
    /// Environment ID (optional)
    pub environment_id: Option<String>,
    /// Persona ID (optional)
    pub persona_id: Option<String>,
    /// Scenario ID (optional)
    pub scenario_id: Option<String>,
    /// Reality level (optional, 0.0-1.0)
    pub reality_level: Option<f64>,
    /// Metadata to attach
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Request to compare snapshots
#[derive(Debug, Deserialize)]
pub struct CompareSnapshotsRequest {
    /// Left snapshot ID (or use environment/persona/scenario)
    pub left_snapshot_id: Option<String>,
    /// Right snapshot ID (or use environment/persona/scenario)
    pub right_snapshot_id: Option<String>,
    /// Left environment ID (alternative to snapshot_id)
    pub left_environment_id: Option<String>,
    /// Right environment ID (alternative to snapshot_id)
    pub right_environment_id: Option<String>,
    /// Left persona ID (alternative to snapshot_id)
    pub left_persona_id: Option<String>,
    /// Right persona ID (alternative to snapshot_id)
    pub right_persona_id: Option<String>,
    /// Left scenario ID (alternative to snapshot_id)
    pub left_scenario_id: Option<String>,
    /// Right scenario ID (alternative to snapshot_id)
    pub right_scenario_id: Option<String>,
    /// Left reality level (alternative to snapshot_id)
    pub left_reality_level: Option<f64>,
    /// Right reality level (alternative to snapshot_id)
    pub right_reality_level: Option<f64>,
}
