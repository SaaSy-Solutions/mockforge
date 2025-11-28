//! Snapshot types and data structures
//!
//! This module defines the data structures for snapshot manifests, metadata,
//! and component specifications.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Snapshot manifest containing metadata and structure information
///
/// The manifest describes what components are included in a snapshot and
/// provides metadata for identification and validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// Manifest version (for future compatibility)
    pub version: String,
    /// Snapshot name (user-provided identifier)
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Timestamp when snapshot was created
    pub created_at: DateTime<Utc>,
    /// Workspace ID this snapshot belongs to
    pub workspace_id: String,
    /// Components included in this snapshot
    pub components: SnapshotComponents,
    /// Total size of snapshot in bytes
    pub size_bytes: u64,
    /// SHA-256 checksum for validation
    pub checksum: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl SnapshotManifest {
    /// Create a new snapshot manifest
    pub fn new(name: String, workspace_id: String, components: SnapshotComponents) -> Self {
        Self {
            version: "1.0".to_string(),
            name,
            description: None,
            created_at: Utc::now(),
            workspace_id,
            components,
            size_bytes: 0,
            checksum: String::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Components included in a snapshot
///
/// Specifies which parts of the system state are captured in the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotComponents {
    /// Include unified state from consistency engine
    pub unified_state: bool,
    /// Include VBR (Virtual Backend Reality) state
    pub vbr_state: bool,
    /// Include recorder state (recorded requests)
    pub recorder_state: bool,
    /// Include workspace configuration
    pub workspace_config: bool,
    /// List of protocols to include (empty = all)
    pub protocols: Vec<String>,
}

impl Default for SnapshotComponents {
    fn default() -> Self {
        Self {
            unified_state: true,
            vbr_state: false,
            recorder_state: false,
            workspace_config: true,
            protocols: Vec::new(), // Empty = all protocols
        }
    }
}

impl SnapshotComponents {
    /// Create components that include everything
    pub fn all() -> Self {
        Self {
            unified_state: true,
            vbr_state: true,
            recorder_state: true,
            workspace_config: true,
            protocols: Vec::new(),
        }
    }

    /// Create components with only unified state
    pub fn unified_state_only() -> Self {
        Self {
            unified_state: true,
            vbr_state: false,
            recorder_state: false,
            workspace_config: false,
            protocols: Vec::new(),
        }
    }
}

/// Snapshot metadata for listing and querying
///
/// Lightweight representation of a snapshot for listing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Snapshot name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Workspace ID
    pub workspace_id: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Size in bytes
    pub size_bytes: u64,
    /// Components included
    pub components: SnapshotComponents,
}

impl From<SnapshotManifest> for SnapshotMetadata {
    fn from(manifest: SnapshotManifest) -> Self {
        Self {
            name: manifest.name,
            description: manifest.description,
            workspace_id: manifest.workspace_id,
            created_at: manifest.created_at,
            size_bytes: manifest.size_bytes,
            components: manifest.components,
        }
    }
}
