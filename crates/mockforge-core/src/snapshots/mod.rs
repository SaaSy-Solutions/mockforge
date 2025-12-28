//! Time travel and snapshot functionality
//!
//! This module provides functionality to save and restore entire system states
//! (across protocols, personas, and reality level) with snapshots.

pub mod manager;
pub mod state_exporter;
pub mod types;

pub use manager::SnapshotManager;
pub use state_exporter::{BoxedStateExporter, ProtocolStateExporter};
pub use types::{SnapshotComponents, SnapshotManifest, SnapshotMetadata};
