//! Protocol adapters for consistency engine
//!
//! Protocol adapters implement the ProtocolAdapter trait to integrate
//! individual protocols (HTTP, GraphQL, gRPC, etc.) with the consistency engine.
//!
//! Each adapter listens to state change events and updates its protocol-specific
//! state accordingly, ensuring all protocols reflect the unified state.

use crate::consistency::{PersonaProfile, StateChangeEvent};
use crate::protocol_abstraction::Protocol;
use crate::Result;

/// Trait for protocol adapters
///
/// Protocol adapters integrate individual protocols with the consistency engine.
/// They receive state change events and update their protocol-specific state
/// to reflect the unified state.
#[async_trait::async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// Get the protocol this adapter handles
    fn protocol(&self) -> Protocol;

    /// Handle a state change event
    ///
    /// Called by the consistency engine when state changes. The adapter
    /// should update its internal state to reflect the change.
    async fn on_state_change(&self, event: &StateChangeEvent) -> Result<()>;

    /// Get current protocol state
    ///
    /// Returns the current state of this protocol for the given workspace,
    /// or None if the workspace doesn't exist or has no state for this protocol.
    async fn get_current_state(
        &self,
        workspace_id: &str,
    ) -> Result<Option<crate::consistency::types::ProtocolState>>;

    /// Apply persona to this protocol
    ///
    /// Called when a persona is set for a workspace. The adapter should
    /// update its handlers/middleware to use this persona for data generation.
    async fn apply_persona(&self, workspace_id: &str, persona: &PersonaProfile) -> Result<()>;

    /// Apply scenario to this protocol
    ///
    /// Called when a scenario is set for a workspace. The adapter should
    /// update its state machine or workflow to reflect this scenario.
    async fn apply_scenario(&self, workspace_id: &str, scenario_id: &str) -> Result<()>;
}

// Placeholder adapters will be implemented in protocol-specific crates
// (mockforge-http, mockforge-graphql, etc.)
