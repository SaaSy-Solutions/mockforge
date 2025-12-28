//! Protocol state exporter trait for snapshot functionality
//!
//! This module provides a trait that protocol engines can implement to allow
//! their state to be captured in snapshots.

use async_trait::async_trait;
use serde_json::Value;

/// Trait for protocol engines that can export their state for snapshots
///
/// Protocol engines (VBR, Recorder, GraphQL cache, etc.) can implement this trait
/// to enable their state to be captured and restored as part of snapshots.
#[async_trait]
pub trait ProtocolStateExporter: Send + Sync {
    /// Get the protocol name (e.g., "vbr", "recorder", "graphql")
    fn protocol_name(&self) -> &str;

    /// Export the current state as JSON
    ///
    /// This should capture all relevant state that needs to be persisted
    /// for later restoration.
    async fn export_state(&self) -> crate::Result<Value>;

    /// Import state from JSON
    ///
    /// This should restore the protocol engine to the state captured
    /// in the provided JSON value.
    async fn import_state(&self, state: Value) -> crate::Result<()>;

    /// Get a summary of the current state (for display/logging)
    ///
    /// Returns a short description of the state, e.g., "5 entities, 1234 records"
    async fn state_summary(&self) -> String {
        "state available".to_string()
    }
}

/// A type-erased wrapper for protocol state exporters
///
/// This wrapper allows storing different protocol engines in the same collection
/// while preserving their state export capabilities.
pub struct BoxedStateExporter(pub Box<dyn ProtocolStateExporter>);

impl BoxedStateExporter {
    /// Create a new boxed state exporter
    pub fn new<T: ProtocolStateExporter + 'static>(exporter: T) -> Self {
        Self(Box::new(exporter))
    }
}

#[async_trait]
impl ProtocolStateExporter for BoxedStateExporter {
    fn protocol_name(&self) -> &str {
        self.0.protocol_name()
    }

    async fn export_state(&self) -> crate::Result<Value> {
        self.0.export_state().await
    }

    async fn import_state(&self, state: Value) -> crate::Result<()> {
        self.0.import_state(state).await
    }

    async fn state_summary(&self) -> String {
        self.0.state_summary().await
    }
}
